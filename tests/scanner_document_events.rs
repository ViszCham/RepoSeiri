use seiri_core::{
    DocumentDiagnosticKind, DocumentEvent, DocumentScan, EvidenceAtom, EvidenceSourceSpan,
    ImportantFileKind, MarkdownEvidenceKind, SourceSpan,
};
use seiri_fs::{IgnorePolicy, ScanOptions, WalkCompletion, WalkLimitKind};
use seiri_markdown::{DocumentScanOptions, MarkdownError};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn bounded_walker_separates_ignore_and_classification() {
    let repo = TempRepo::new("bounded-walker");
    repo.write("README.md", "# Fixture\n");
    repo.write("docs/guide.md", "guide\n");
    repo.write("target/generated.txt", "ignored\n");
    repo.write("custom-cache/ignored.txt", "ignored\n");

    let options = ScanOptions {
        ignore_policy: IgnorePolicy::with_additional_names(vec!["custom-cache".to_string()]),
        ..ScanOptions::default()
    };
    assert_eq!(
        options.ignore_policy.additional_names(),
        &["custom-cache".to_string()]
    );
    let walk = seiri_fs::walk_repository_with_options(repo.path(), &options).expect("walk repo");
    assert_eq!(walk.root().as_path(), repo.canonical_path());
    assert_eq!(walk.summary().visited_entries, walk.records().len());
    assert_eq!(walk.summary().ignored_entries, 2);
    assert!(walk
        .records()
        .iter()
        .all(|record| !record.path.starts_with("target")));
    assert!(walk
        .records()
        .iter()
        .all(|record| !record.path.starts_with("custom-cache")));

    let scan = seiri_fs::scan_repository_with_options(repo.path(), &options).expect("scan repo");
    assert_eq!(scan.walk_summary, *walk.summary());
    assert!(scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::Readme));
    assert!(scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::DocsDirectory));
}

#[test]
fn walker_limits_return_typed_partial_records() {
    let repo = TempRepo::new("walker-limits");
    repo.write("README.md", "# Fixture\n");
    repo.write("one.txt", "1\n");
    repo.write("nested/two.txt", "2\n");

    let entry_partial = seiri_fs::walk_repository_with_options(
        repo.path(),
        &ScanOptions {
            max_entries: 1,
            ..ScanOptions::default()
        },
    )
    .expect("entry limit returns a partial walk");
    assert!(matches!(
        entry_partial.summary().completion,
        WalkCompletion::Truncated(ref truncation)
            if matches!(truncation.kind, WalkLimitKind::Entries)
                && truncation.limit == 1
    ));
    assert!(entry_partial.records().len() <= 1);

    let depth_partial = seiri_fs::walk_repository_with_options(
        repo.path(),
        &ScanOptions {
            max_depth: 0,
            ..ScanOptions::default()
        },
    )
    .expect("depth limit returns a partial walk");
    assert!(matches!(
        depth_partial.summary().completion,
        WalkCompletion::Truncated(ref truncation)
            if matches!(truncation.kind, WalkLimitKind::Depth)
                && truncation.limit == 0
    ));
    assert!(depth_partial
        .records()
        .iter()
        .all(|record| record.path != "nested/two.txt"));

    let directory_partial = seiri_fs::walk_repository_with_options(
        repo.path(),
        &ScanOptions {
            max_directory_entries: 2,
            ..ScanOptions::default()
        },
    )
    .expect("directory limit returns a partial walk");
    assert!(matches!(
        directory_partial.summary().completion,
        WalkCompletion::Truncated(ref truncation)
            if matches!(truncation.kind, WalkLimitKind::DirectoryEntries)
                && truncation.path == "."
                && truncation.limit == 2
    ));
    assert!(
        directory_partial.records().is_empty(),
        "partial contents of an oversized directory must not become evidence"
    );
}

#[test]
fn document_events_keep_utf8_spans_and_soft_diagnostics() {
    let source = "# 使用方法\n\n[文書](docs/guide.md)\n![CI](https://img.shields.io/badge/ci-ok)\n[broken](target\n";
    let document = seiri_markdown::scan_document("README.md", source).expect("scan document");

    assert_eq!(document.source_bytes(), source.len());
    assert!(!document.events().is_empty());
    assert_eq!(document.diagnostics().len(), 1);
    assert_eq!(
        document.diagnostics()[0].kind,
        DocumentDiagnosticKind::UnclosedLinkTarget
    );
    for span in document
        .events()
        .iter()
        .map(|event| event.span().expect("event span"))
        .chain(document.diagnostics().iter().map(|value| value.span))
    {
        assert!(source.get(span.byte_start..span.byte_end).is_some());
    }
    assert!(document.events().windows(2).all(|pair| {
        let left = pair[0].span().expect("left span");
        let right = pair[1].span().expect("right span");
        (left.byte_start, pair[0].order_rank(), left.byte_end)
            <= (right.byte_start, pair[1].order_rank(), right.byte_end)
    }));

    let summary = seiri_markdown::summarize_readme_document(&document, None);
    assert_eq!(summary, seiri_markdown::parse_readme("README.md", source));
    let serialized = serde_json::to_value(&document).expect("document JSON");
    assert!(serialized.get("source").is_none());
    assert_eq!(serialized["source_bytes"], source.len());
}

#[test]
fn document_limits_and_invalid_utf8_are_typed_failures() {
    let source_error = seiri_markdown::scan_document_with_options(
        "README.md",
        "# Docs\n",
        &DocumentScanOptions {
            max_source_bytes: 3,
            ..DocumentScanOptions::default()
        },
    )
    .expect_err("source limit must fail");
    assert!(matches!(
        source_error,
        MarkdownError::SourceLimitExceeded { limit: 3, .. }
    ));

    let event_error = seiri_markdown::scan_document_with_options(
        "README.md",
        "# Docs\n",
        &DocumentScanOptions {
            max_events: 1,
            ..DocumentScanOptions::default()
        },
    )
    .expect_err("event limit must fail");
    assert!(matches!(
        event_error,
        MarkdownError::EventLimitExceeded { limit: 1, .. }
    ));

    let diagnostic_error = seiri_markdown::scan_document_with_options(
        "README.md",
        "[broken](target\n",
        &DocumentScanOptions {
            max_diagnostics: 0,
            ..DocumentScanOptions::default()
        },
    )
    .expect_err("diagnostic limit must fail");
    assert!(matches!(
        diagnostic_error,
        MarkdownError::DiagnosticLimitExceeded { limit: 0, .. }
    ));

    let repo = TempRepo::new("invalid-utf8");
    repo.write_bytes("README.md", &[0xff, 0xfe]);
    let utf8_error =
        seiri_markdown::scan_readme_document(repo.path()).expect_err("invalid UTF-8 must fail");
    assert!(matches!(
        utf8_error,
        MarkdownError::InvalidUtf8 { valid_up_to: 0, .. }
    ));

    let bounded_repo = TempRepo::new("bounded-document-session");
    bounded_repo.write(
        "README.md",
        "# This source is larger than the configured cap\n",
    );
    let fs_scan = seiri_fs::scan_repository(bounded_repo.path()).expect("scan bounded repo");
    let session = seiri_markdown::scan_document_source_session_with_options_and_scope(
        bounded_repo.path(),
        &fs_scan.files,
        true,
        &seiri_markdown::DocumentIndexOptions {
            document: DocumentScanOptions {
                max_source_bytes: 8,
                ..DocumentScanOptions::default()
            },
            ..seiri_markdown::DocumentIndexOptions::default()
        },
        None,
    )
    .expect("oversized source produces a typed index state");
    assert!(session.sources().is_empty());
    assert_eq!(
        session.index().entries()[0].status,
        seiri_core::DocumentScanStatus::SkippedByteBudget
    );
}

#[test]
fn document_scan_wire_rejects_missing_bounds_and_order() {
    let source = "# Docs\n\n# Security\n";
    let document = seiri_markdown::scan_document("README.md", source).expect("scan document");

    let mut missing_span = serde_json::to_value(&document).expect("document JSON");
    missing_span["events"][0]["data"]["span"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<DocumentScan>(missing_span).is_err());

    let mut out_of_bounds = serde_json::to_value(&document).expect("document JSON");
    out_of_bounds["events"][0]["data"]["span"]["byte_end"] = serde_json::json!(source.len() + 1);
    assert!(serde_json::from_value::<DocumentScan>(out_of_bounds).is_err());

    let mut reordered = serde_json::to_value(&document).expect("document JSON");
    reordered["events"]
        .as_array_mut()
        .expect("events array")
        .reverse();
    assert!(serde_json::from_value::<DocumentScan>(reordered).is_err());

    let diagnostic_document =
        seiri_markdown::scan_document("README.md", "[a](x\n[b](y\n").expect("diagnostics");
    let mut reordered_diagnostics =
        serde_json::to_value(&diagnostic_document).expect("document JSON");
    reordered_diagnostics["diagnostics"]
        .as_array_mut()
        .expect("diagnostics array")
        .reverse();
    assert!(serde_json::from_value::<DocumentScan>(reordered_diagnostics).is_err());
}

#[test]
fn audit_uses_document_events_as_canonical_evidence_input() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let document = snapshot
        .readme_document
        .as_ref()
        .expect("README document scan");
    let summary = snapshot.readme_summary.as_ref().expect("README summary");

    assert_eq!(
        document.events().len(),
        document
            .events()
            .iter()
            .filter(|event| matches!(event, seiri_core::DocumentEvent::VisibleProse(_)))
            .count()
            + summary.headings.len()
            + summary.links.len()
            + summary.badges.len()
            + summary.route_candidates.len()
    );
    let expected_markdown_fact_count = snapshot
        .document_index
        .scanned_documents()
        .map(|indexed| {
            indexed
                .scan
                .as_ref()
                .expect("scanned index entry")
                .events()
                .len()
        })
        .sum::<usize>();
    let markdown_fact_count = snapshot
        .evidence_kernel
        .facts()
        .iter()
        .filter(|fact| matches!(fact.atom, EvidenceAtom::Markdown { .. }))
        .count();
    assert_eq!(markdown_fact_count, expected_markdown_fact_count);
    assert!(document.events().iter().all(|event| {
        let span = event.span();
        snapshot.evidence_kernel.facts().iter().any(|fact| {
            spans_equal(fact.provenance.span, span) && event_matches_fact(event, fact.atom)
        })
    }));

    let json = seiri_report::to_json(&snapshot).expect("snapshot JSON");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse snapshot JSON");
    assert!(value["readme_document"]["events"].is_array());
    assert!(value.get("readme").is_none());
}

fn event_matches_fact(event: &DocumentEvent, atom: EvidenceAtom) -> bool {
    matches!(
        (event, atom),
        (
            DocumentEvent::VisibleProse(_),
            EvidenceAtom::Markdown {
                event: MarkdownEvidenceKind::VisibleProse,
                ..
            }
        ) | (
            DocumentEvent::Heading(_),
            EvidenceAtom::Markdown {
                event: MarkdownEvidenceKind::Heading,
                ..
            }
        ) | (
            DocumentEvent::Link(_),
            EvidenceAtom::Markdown {
                event: MarkdownEvidenceKind::Link,
                ..
            }
        ) | (
            DocumentEvent::Badge(_),
            EvidenceAtom::Markdown {
                event: MarkdownEvidenceKind::Badge,
                ..
            }
        ) | (
            DocumentEvent::RouteCandidate(_),
            EvidenceAtom::Markdown {
                event: MarkdownEvidenceKind::RouteCandidate,
                ..
            }
        )
    )
}

fn spans_equal(actual: Option<EvidenceSourceSpan>, expected: Option<SourceSpan>) -> bool {
    match (actual, expected) {
        (Some(actual), Some(expected)) => {
            actual.line.get() == expected.line as u32
                && actual.column.get() == expected.column as u32
                && actual.byte_start.get() == expected.byte_start as u32
                && actual.byte_end.get() == expected.byte_end as u32
        }
        (None, None) => true,
        _ => false,
    }
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "reposeiri-scanner-events-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp repo");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn canonical_path(&self) -> PathBuf {
        fs::canonicalize(&self.path).expect("canonical temp repo")
    }

    fn write(&self, relative: &str, content: &str) {
        self.write_bytes(relative, content.as_bytes());
    }

    fn write_bytes(&self, relative: &str, content: &[u8]) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, content).expect("write fixture file");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
