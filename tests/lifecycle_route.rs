use seiri_core::{GateKind, PatternGroup, RouteKind, RouteState, ROUTE_MEANING_ROUTES};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn q3_lifecycle_classification_keeps_support_and_release_boundaries() {
    assert_eq!(
        seiri_markdown::classify_route("Supported versions", None),
        RouteKind::Lifecycle
    );
    assert_eq!(
        seiri_markdown::classify_route("Compatibility policy", None),
        RouteKind::Lifecycle
    );
    assert_eq!(
        seiri_markdown::classify_route("Version support", Some("docs/lifecycle.md")),
        RouteKind::Lifecycle
    );
    assert_eq!(
        seiri_markdown::classify_route("Support", None),
        RouteKind::Support
    );
    assert_eq!(
        seiri_markdown::classify_route("Release notes", None),
        RouteKind::Release
    );
}

#[test]
fn q3_readme_route_map_verifies_lifecycle_targets() {
    let summary = seiri_markdown::analyze_readme(fixture("lifecycle-route-repo"))
        .expect("read README")
        .expect("README exists");

    let lifecycle = summary
        .route_map
        .entries
        .iter()
        .find(|entry| entry.route == RouteKind::Lifecycle)
        .expect("lifecycle route entry");

    assert_eq!(lifecycle.state, RouteState::Verified);
    assert_eq!(lifecycle.target_count, 1);
    assert!(lifecycle.reason.contains("lifecycle"));
    assert!(summary
        .route_candidates
        .iter()
        .any(|candidate| candidate.route == RouteKind::Lifecycle));
}

#[test]
fn q3_lifecycle_patterns_are_manual_baseline_and_candidate_surfaces() {
    let registry = seiri_patterns::common_registry();
    let baseline = registry
        .definitions()
        .iter()
        .find(|definition| definition.id == "common.lifecycle.route_present")
        .expect("lifecycle baseline definition");
    assert_eq!(baseline.group, PatternGroup::Lif);
    assert_eq!(baseline.route, Some(RouteKind::Lifecycle));
    assert_eq!(
        baseline.adoption_stage,
        seiri_patterns::PatternAdoptionStage::CommonBaseline
    );
    assert_eq!(baseline.boundary.missing_gate, GateKind::Manual);

    let lifecycle = registry
        .definitions()
        .iter()
        .find(|definition| definition.id == "LIF-001")
        .expect("lifecycle candidate definition");

    assert_eq!(lifecycle.group, PatternGroup::Lif);
    assert_eq!(lifecycle.route, Some(RouteKind::Lifecycle));
    assert_eq!(
        lifecycle.adoption_stage,
        seiri_patterns::PatternAdoptionStage::Candidate
    );
    assert_eq!(lifecycle.boundary.missing_gate, GateKind::Manual);
    assert!(registry
        .evaluation_definitions()
        .iter()
        .any(|definition| definition.id == "common.lifecycle.route_present"));
    assert!(!registry
        .evaluation_definitions()
        .iter()
        .any(|definition| definition.id == "LIF-001"));
}

#[test]
fn q3_lifecycle_route_state_is_manual_when_missing() {
    let snapshot =
        seiri_report::audit_repository(fixture("missing-readme-repo")).expect("audit fixture");

    let state = snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Lifecycle)
        .expect("lifecycle route state");
    assert_eq!(state.state, RouteState::UnsafeToInvent);

    let priority = snapshot
        .missing_route_priority
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Lifecycle)
        .expect("lifecycle priority");
    assert_eq!(priority.gate, GateKind::Manual);
    assert_eq!(priority.calibration_estimate, None);
    assert!(priority
        .baseline_pattern_ids
        .contains(&"common.lifecycle.route_present".to_string()));
    assert!(priority
        .candidate_pattern_ids
        .contains(&"LIF-001".to_string()));
}

#[test]
fn q3_lifecycle_is_covered_by_route_meaning_registry() {
    assert!(ROUTE_MEANING_ROUTES.contains(&RouteKind::Lifecycle));
    let rule = seiri_core::route_meaning_rule(RouteKind::Lifecycle, RouteState::Verified);
    assert!(rule
        .does_not_indicate
        .contains(&seiri_core::ClaimBoundaryKind::NotMaintenanceGuarantee));
    assert!(rule
        .does_not_indicate
        .contains(&seiri_core::ClaimBoundaryKind::NotAutomaticPolicyAdoption));
}
