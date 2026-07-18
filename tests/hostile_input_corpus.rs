use seiri_core::{
    PatchProposal, PatchProposalDecision, PatchProposalIssueKind, PatchTextEdit, TextDocumentBase,
    TextEditSpan,
};
use seiri_patterns::{PredicateInstruction, PredicateProgram};
use std::fs;
use std::io::Cursor;

#[test]
fn deterministic_hostile_corpus_is_bounded_and_panic_free() {
    let deep_markdown = format!("{}x{}\n", "[".repeat(4_096), "]".repeat(4_096));
    let markdown = seiri_markdown::scan_document_with_options(
        "README.md",
        &deep_markdown,
        &seiri_markdown::DocumentScanOptions {
            max_source_bytes: 32 * 1024,
            max_events: 64,
            max_diagnostics: 8,
        },
    );
    match markdown {
        Ok(scan) => {
            assert_eq!(scan.path(), "README.md");
            assert_eq!(scan.source_bytes(), deep_markdown.len());
            assert!(scan.events().len() <= 64);
            assert!(scan.diagnostics().len() <= 8);
            assert!(scan.events().iter().all(|event| {
                event
                    .span()
                    .is_some_and(|span| span.byte_end <= deep_markdown.len())
            }));
        }
        Err(seiri_markdown::MarkdownError::EventLimitExceeded { path, limit }) => {
            assert_eq!(path, "README.md");
            assert_eq!(limit, 64);
        }
        Err(seiri_markdown::MarkdownError::DiagnosticLimitExceeded { path, limit }) => {
            assert_eq!(path, "README.md");
            assert_eq!(limit, 8);
        }
        Err(error) => panic!("hostile Markdown violated the bounded-result contract: {error}"),
    }

    assert!(
        PredicateProgram::try_new(Vec::new(), vec![PredicateInstruction::PushAtom(u8::MAX)])
            .is_err()
    );
    assert!(PredicateProgram::try_new(Vec::new(), Vec::new()).is_err());

    let source = "aéz\n".as_bytes();
    let non_boundary = PatchProposal::new(
        "hostile-span",
        "README.md",
        TextDocumentBase::from_bytes(source),
        vec![PatchTextEdit::literal(
            "edit",
            TextEditSpan::new(2, source.len() + 1).expect("ordered span"),
            "x",
        )],
    );
    let preflight = non_boundary.preflight_against(source);
    assert_ne!(preflight.decision, PatchProposalDecision::Ready);
    assert!(preflight.has_issue(PatchProposalIssueKind::SpanOutOfBounds));

    let limits =
        seiri_calibration::StreamingCalibrationLimits::new(16, 1, 1, 1).expect("non-zero limits");
    let metadata =
        seiri_calibration::StreamingCalibrationMetadata::new("hostile", "hostile", "unknown");
    let oversized = vec![b'x'; 17];
    assert!(seiri_calibration::calibrate_jsonl_reader_with_limits(
        Cursor::new(oversized),
        metadata,
        limits
    )
    .is_err());
    assert!(seiri_calibration::StreamingCalibrationLimits::new(0, 1, 1, 1).is_none());
}

#[test]
fn repository_audit_does_not_write_or_emit_unbounded_output() {
    let root = tempfile::tempdir().expect("repository");
    fs::create_dir_all(root.path().join(".github/ISSUE_TEMPLATE")).expect("github");
    fs::write(root.path().join("README.md"), "# Tool\n").expect("README");
    fs::write(
        root.path().join(".github/ISSUE_TEMPLATE/hostile.yml"),
        format!(
            "name: hostile\ndescription: {}\nbody:\n  - type: input\n    id: x\n    attributes:\n      label: x\n",
            "x".repeat(16 * 1024)
        ),
    )
    .expect("issue form");
    fs::write(
        root.path().join(".github/CODEOWNERS"),
        format!("{} @owner\n", "a".repeat(16 * 1024)),
    )
    .expect("CODEOWNERS");

    let before = repository_files(root.path());
    let snapshot = seiri_report::audit_repository(root.path()).expect("bounded audit");
    let output = seiri_report::to_json(&snapshot).expect("JSON");
    let after = repository_files(root.path());
    assert_eq!(before, after);
    assert!(output.len() < 2 * 1024 * 1024);
    assert_eq!(
        snapshot.remote_evidence.status,
        seiri_core::RemoteEvidenceStatus::NotRequested
    );
}

fn repository_files(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    fn visit(root: &std::path::Path, current: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .expect("read directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("directory entries");
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root)
                        .expect("relative")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).expect("file"),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files
}
