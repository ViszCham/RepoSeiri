use seiri_core::{
    CoverageIncompleteReason, CoverageScope, CoverageStatus, DocumentRole, DocumentScanStatus,
    DocumentScopeClass, Observation, ProfileKind, UnknownReason,
};
use seiri_fs::ScanOptions;
use seiri_markdown::DocumentIndexOptions;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn document_index_bounds_scan_budget_and_preserves_role_coverage() {
    let repo = TempRepo::new("document-budget");
    repo.write("README.md", "# Purpose\n");
    repo.write("docs/guide.md", "# Documentation\n");

    let fs_scan = seiri_fs::scan_repository(repo.path()).expect("filesystem scan");
    let index = seiri_markdown::scan_document_index_with_options(
        repo.path(),
        &fs_scan.files,
        true,
        &DocumentIndexOptions {
            max_total_source_bytes: "# Purpose\n".len(),
            ..DocumentIndexOptions::default()
        },
    )
    .expect("document index");

    assert!(index
        .entries()
        .iter()
        .any(|entry| entry.role == DocumentRole::RootReadme && entry.scan.is_some()));
    assert!(index.entries().iter().any(|entry| {
        entry.path == "docs/guide.md" && entry.status == DocumentScanStatus::SkippedByteBudget
    }));
    assert_eq!(
        index.coverage_for_role(DocumentRole::Documentation),
        Some(CoverageStatus::Partial(
            CoverageIncompleteReason::LimitExceeded
        ))
    );
}

#[test]
fn document_index_prioritizes_core_roles_and_readme_targets_before_path_order() {
    let repo = TempRepo::new("document-priority");
    repo.write("README.md", "# Tool\n\n[Deep guide](z-last/guide.md)\n");
    repo.write("SECURITY.md", "# Security\n");
    repo.write("SUPPORT.md", "# Support\n");
    repo.write("docs/index.md", "# Documentation\n");
    repo.write("z-last/guide.md", "# Deep guide\n");
    for index in 0..40 {
        repo.write(&format!("a-noise/{index:02}.md"), "# Noise\n");
    }

    let fs_scan = seiri_fs::scan_repository(repo.path()).expect("filesystem scan");
    let index = seiri_markdown::scan_document_index_with_options(
        repo.path(),
        &fs_scan.files,
        true,
        &DocumentIndexOptions {
            max_documents: 5,
            ..DocumentIndexOptions::default()
        },
    )
    .expect("document index");

    for path in [
        "README.md",
        "SECURITY.md",
        "SUPPORT.md",
        "docs/index.md",
        "z-last/guide.md",
    ] {
        assert!(
            index
                .entries()
                .iter()
                .any(|entry| entry.path == path && entry.scan.is_some()),
            "priority document {path} was not selected"
        );
    }
    assert_eq!(index.selection().selected, 5);
    assert!(index.selection().skipped_document_budget >= 40);
}

#[test]
fn scope_class_separates_supporting_documents_from_repository_content() {
    let repo = TempRepo::new("document-scope");
    repo.write("README.md", "# Tool\n");
    repo.write("fixtures/example/docs/guide.md", "<table><tr><td>broken\n");

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit");
    let fixture = snapshot
        .document_index
        .entries()
        .iter()
        .find(|entry| entry.path == "fixtures/example/docs/guide.md")
        .expect("fixture document");
    assert_eq!(fixture.scope_class, DocumentScopeClass::Fixture);

    let docs_slot = snapshot
        .route_content
        .assessments
        .iter()
        .find(|assessment| assessment.code == "docs.user_guide")
        .expect("docs slot");
    assert!(!matches!(
        docs_slot.observation,
        Observation::Unknown(UnknownReason::UnsupportedSyntax)
    ));
}

#[test]
fn document_parse_failure_is_indexed_as_unknown_coverage() {
    let repo = TempRepo::new("invalid-markdown");
    repo.write("README.md", "# Purpose\n");
    repo.write_bytes("docs/invalid.md", &[0xff, 0xfe]);

    let fs_scan = seiri_fs::scan_repository(repo.path()).expect("filesystem scan");
    let index = seiri_markdown::scan_document_index(repo.path(), &fs_scan.files, true)
        .expect("document index");

    assert!(index.entries().iter().any(|entry| {
        entry.path == "docs/invalid.md" && entry.status == DocumentScanStatus::InvalidUtf8
    }));
    assert_eq!(
        index.coverage_for_role(DocumentRole::Documentation),
        Some(CoverageStatus::Partial(
            CoverageIncompleteReason::InvalidUtf8
        ))
    );
}

#[test]
fn partial_report_coverage_never_converts_missing_content_to_absence() {
    let repo = TempRepo::new("partial-report");
    repo.write("README.md", "# Purpose\n");
    repo.write("zeta.txt", "extra\n");

    let snapshot = seiri_report::audit_repository_with_options(
        repo.path(),
        ProfileKind::Common,
        &ScanOptions {
            max_entries: 1,
            ..ScanOptions::default()
        },
        &DocumentIndexOptions::default(),
    )
    .expect("partial audit");

    assert_eq!(
        snapshot
            .coverage
            .observe_absence::<()>(CoverageScope::RepositoryFiles),
        Observation::Unknown(UnknownReason::LimitExceeded)
    );
    let docs_concept = snapshot
        .route_content
        .assessments
        .iter()
        .find(|assessment| assessment.code == "docs.user_guide")
        .expect("docs concept observation");
    assert_eq!(
        docs_concept.observation,
        Observation::Unknown(UnknownReason::LimitExceeded)
    );
}

#[test]
fn route_content_keeps_presence_separate_from_adequacy() {
    let repo = TempRepo::new("route-content");
    repo.write(
        "README.md",
        "# Purpose\n\
         ## Audience scope\n\
         ## Documentation guide\n\
         ## Architecture\n\
         ## Install\n\
         ## Getting started\n\
         ## Support questions\n\
         ## Contact response\n\
         ## Reproduction environment\n\
         ## Security disclosure\n\
         ## Contributing development\n\
         ## Test check\n\
         ## Report vulnerability\n\
         ## Security policy scope\n\
         ## Changelog changes\n\
         ## Compatibility migration\n\
         ## Maintenance status\n\
         ## Deprecation plan\n\
         ## Governance decision\n\
         ## Maintainer role\n\
         ## License permission\n\
         ## Critical path ownership\n\
         ## Formatting style\n\
         ![CI](https://img.shields.io/badge/ci-ok)\n",
    );
    repo.write("LICENSE", "placeholder\n");
    repo.write(".gitignore", "target/\n");
    repo.write(".github/workflows/ci.yml", "name: ci\n");
    repo.write(".github/CODEOWNERS", "* @owner\n");

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit repository");
    let workflow = snapshot
        .document_index
        .entries()
        .iter()
        .find(|entry| entry.path == ".github/workflows/ci.yml")
        .expect("workflow config candidate");
    assert_eq!(workflow.status, DocumentScanStatus::NotMarkdown);
    assert!(workflow.document_id.is_some());
    assert!(workflow.digest.is_some());
    assert!(workflow.encoding.is_some());
    assert_eq!(
        snapshot.route_content.assessments.len(),
        seiri_core::route_content_contract().len()
    );
    for code in [
        "identity.purpose",
        "docs.user_guide",
        "quickstart.install",
        "security.private_disclosure",
        "governance.decision",
    ] {
        let assessment = snapshot
            .route_content
            .assessments
            .iter()
            .find(|assessment| assessment.code == code)
            .expect("content slot");
        assert!(
            matches!(assessment.observation, Observation::Present { .. }),
            "expected {assessment:?} to be observed"
        );
    }
    let docs_version = snapshot
        .route_content
        .assessments
        .iter()
        .find(|assessment| assessment.code == "docs.version")
        .expect("docs version slot");
    assert!(matches!(
        docs_version.observation,
        Observation::Absent { .. }
    ));
    assert!(snapshot
        .route_content
        .assessments
        .iter()
        .any(
            |assessment| assessment.code == "security.private_disclosure"
                && matches!(assessment.observation, Observation::Present { .. })
        ));

    let json = seiri_report::to_json(&snapshot).expect("canonical JSON");
    let wire: serde_json::Value = serde_json::from_str(&json).expect("wire JSON");
    assert!(wire.get("document_index").is_some());
    assert!(wire.get("route_content").is_some());
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
            "reposeiri-local-evidence-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp repo");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, content: &str) {
        self.write_bytes(relative, content.as_bytes());
    }

    fn write_bytes(&self, relative: &str, content: &[u8]) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, content).expect("write fixture");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
