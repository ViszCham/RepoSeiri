use seiri_core::{EvidenceKind, RouteKind};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn report_schema_is_stable_for_missing_readme_fixture() {
    let snapshot =
        seiri_report::audit_repository(fixture("missing-readme-repo")).expect("audit fixture");

    assert_eq!(snapshot.schema_version, "seiri.block_f.v1");
    assert!(snapshot.readme.is_none());
    assert!(snapshot.baseline.is_some());
    assert!(snapshot.profile.is_some());
    assert!(!snapshot.pattern_matches.is_empty());
    assert!(snapshot
        .evidence
        .iter()
        .any(|evidence| evidence.kind == EvidenceKind::ReadmeMissing));
    assert!(snapshot
        .findings
        .iter()
        .any(|finding| finding.title == "README is missing"));

    let json = seiri_report::to_json(&snapshot).expect("render JSON");
    let parsed = serde_json::from_str::<Value>(&json).expect("valid JSON");
    assert_eq!(parsed["schema_version"], "seiri.block_f.v1");
    assert_eq!(parsed["readme"], Value::Null);
    assert!(parsed["baseline"].is_object());
    assert!(parsed["profile"].is_object());
}

#[test]
fn report_distinguishes_readme_routes_from_absent_routes() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");

    assert!(snapshot.readme.is_some());
    assert!(snapshot
        .evidence
        .iter()
        .any(|evidence| evidence.route == Some(RouteKind::Docs)));
    assert!(!snapshot
        .findings
        .iter()
        .any(|finding| finding.title == "README is missing"));

    let markdown = seiri_report::to_markdown(&snapshot);
    assert!(markdown.contains("# RepoSeiri Report"));
    assert!(markdown.contains("## Common Baseline"));
    assert!(markdown.contains("## Profile"));
    assert!(markdown.contains("## Pattern Matches"));
    assert!(markdown.contains("## README"));
    assert!(markdown.contains("## Findings"));
}
