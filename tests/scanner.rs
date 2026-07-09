use seiri_core::ImportantFileKind;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn scanner_detects_important_files_and_ignores_build_outputs() {
    let scan = seiri_fs::scan_repository(fixture("readme-route-repo")).expect("scan fixture");

    assert!(scan.files.iter().any(|record| record.path == "README.md"));
    assert!(scan.files.iter().any(|record| record.path == "docs"));
    assert!(scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::Readme && file.path == "README.md"));
    assert!(scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::DocsDirectory && file.path == "docs"));
    assert!(scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::Workflow));
}

#[test]
fn scanner_keeps_missing_readme_visible() {
    let scan = seiri_fs::scan_repository(fixture("missing-readme-repo")).expect("scan fixture");

    assert!(!scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::Readme));
    assert!(scan
        .important_files
        .iter()
        .any(|file| file.kind == ImportantFileKind::License));
}
