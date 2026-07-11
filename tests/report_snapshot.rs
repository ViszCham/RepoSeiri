use seiri_core::{EvidenceKind, EvidenceScope, RouteKind, RouteState};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn report_schema_is_stable_for_missing_readme_fixture() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("missing-readme-repo"))
        .expect("audit fixture");

    assert_eq!(snapshot.schema_version, "seiri.block_p.v1");
    assert!(snapshot.readme.is_none());
    assert!(snapshot.baseline.is_some());
    assert!(snapshot.profile.is_some());
    assert!(!snapshot.pattern_matches.is_empty());
    assert!(!snapshot.evidence_ledger.is_empty());
    assert!(!snapshot.route_states.is_empty());
    assert!(!snapshot.missing_route_priority.priorities.is_empty());
    assert!(!snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .is_empty());
    assert!(snapshot
        .evidence
        .iter()
        .any(|evidence| evidence.kind == EvidenceKind::ReadmeMissing));
    assert!(snapshot
        .evidence_ledger
        .iter()
        .any(|record| record.kind == EvidenceKind::ReadmeMissing
            && record.scope == EvidenceScope::Root));
    assert!(snapshot
        .route_states
        .iter()
        .any(|state| state.route == RouteKind::License && state.state == RouteState::Structured));
    assert!(snapshot
        .findings
        .iter()
        .any(|finding| finding.title == "README is missing"));

    let json = seiri_report::to_json(&snapshot).expect("render JSON");
    let parsed = serde_json::from_str::<Value>(&json).expect("valid JSON");
    assert_eq!(parsed["schema_version"], "seiri.block_p.v1");
    assert_eq!(parsed["readme"], Value::Null);
    assert!(parsed["evidence_ledger"].is_array());
    assert!(parsed["route_states"].is_array());
    assert!(parsed["missing_route_priority"]["priorities"].is_array());
    assert!(parsed["missing_route_priority"]["co_occurrence_gaps"].is_array());
    assert!(parsed["profile"]["branches"].is_array());
    assert!(parsed["profile"]["branch_summary"].is_object());
    assert!(parsed["baseline"].is_object());
    assert!(parsed["profile"].is_object());
}

#[test]
fn report_distinguishes_readme_routes_from_absent_routes() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");

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
    assert!(markdown.contains("## Route Review v2"));
    assert!(markdown.contains("### Strong Routes"));
    assert!(markdown.contains("### Weak Routes"));
    assert!(markdown.contains("### Missing Routes"));
    assert!(markdown.contains("### Decision Gates"));
    assert!(markdown.contains("## Common Baseline"));
    assert!(markdown.contains("## README Route Map"));
    assert!(markdown.contains("## Route States"));
    assert!(markdown.contains("## Missing Route Priority"));
    assert!(markdown.contains("### Co-occurrence Gaps"));
    assert!(markdown.contains("## Profile"));
    assert!(markdown.contains("### Profile Branch Semantics"));
    assert!(markdown.contains("## Pattern Matches"));
    assert!(markdown.contains("## README"));
    assert!(markdown.contains("## Findings"));
}

#[test]
fn report_verifies_hygiene_when_root_files_and_readme_route_agree() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("hygiene-self-audit-repo"))
        .expect("audit fixture");

    let hygiene = snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Hygiene)
        .expect("hygiene route state");
    assert_eq!(hygiene.state, RouteState::Verified);
    assert!(hygiene.evidence_ids.len() >= 2);

    let registry = seiri_patterns::common_registry();
    let definition = registry
        .definitions()
        .iter()
        .find(|definition| definition.id == "HYG-001")
        .expect("HYG-001");
    let evidence_ids = seiri_patterns::evidence_ids_for_definition(&snapshot, definition);
    assert!(!evidence_ids.is_empty());
}
