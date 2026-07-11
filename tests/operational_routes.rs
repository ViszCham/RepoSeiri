use seiri_core::{ImportantFileKind, RouteKind, RouteState};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn operational_files_feed_route_assessments() {
    let snapshot =
        seiri_report::audit_repository_subtree(fixture("security-support-intake-automation-repo"))
            .expect("audit fixture");

    assert_eq!(snapshot.schema_version, seiri_core::ANALYSIS_SCHEMA_VERSION);
    assert!(snapshot.important_files.iter().any(|file| {
        file.kind == ImportantFileKind::IssueTemplate
            && file.path == ".github/ISSUE_TEMPLATE/feature_request.md"
    }));
    assert!(snapshot.important_files.iter().any(|file| {
        file.kind == ImportantFileKind::IssueForm
            && file.path == ".github/ISSUE_TEMPLATE/bug_report.yml"
    }));
    assert!(snapshot.important_files.iter().any(|file| {
        file.kind == ImportantFileKind::DependencyBot && file.path == ".github/dependabot.yml"
    }));
    assert!(snapshot.important_files.iter().any(|file| {
        file.kind == ImportantFileKind::SecurityAutomation
            && file.path == ".github/workflows/codeql.yml"
    }));

    assert_route_state(&snapshot, RouteKind::Support, RouteState::Verified);
    assert_route_state(&snapshot, RouteKind::Security, RouteState::Verified);
    assert_route_state(&snapshot, RouteKind::Intake, RouteState::Structured);
    assert_route_state(&snapshot, RouteKind::Automation, RouteState::Structured);
}

#[test]
fn candidate_patterns_use_low_level_evidence() {
    let snapshot =
        seiri_report::audit_repository_subtree(fixture("security-support-intake-automation-repo"))
            .expect("audit fixture");
    let registry = seiri_patterns::common_registry();

    for pattern_id in [
        "SUP-001", "SEC-001", "SEC-004", "SEC-007", "INT-003", "INT-010", "AUT-009",
    ] {
        let definition = registry
            .definitions()
            .iter()
            .find(|definition| definition.id == pattern_id)
            .unwrap_or_else(|| panic!("missing pattern {pattern_id}"));
        let evidence_ids = seiri_patterns::evidence_ids_for_definition(&snapshot, definition);
        assert!(
            !evidence_ids.is_empty(),
            "pattern {pattern_id} did not bind evidence"
        );
    }

    assert!(!snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .iter()
        .any(|gap| gap.id == "co-README-SECURITY-CI-DEPENDENCY-BOT"));
}

fn assert_route_state(
    snapshot: &seiri_core::RepositoryAnalysis,
    route: RouteKind,
    state: RouteState,
) {
    assert!(
        snapshot.route_assessments.iter().any(|candidate| {
            candidate.route() == route && candidate.summary_projection().state == state
        }),
        "missing route state {route:?} {state:?}"
    );
}
