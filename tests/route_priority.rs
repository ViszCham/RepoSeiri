use seiri_core::{
    AggregateRepositoryEstimate, CalibrationPriorState, GateKind, MissingRoutePriority,
    RouteCoOccurrenceGap, RouteKind, RouteState,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn standard_missing_route_priority_has_no_aggregate_prior() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("missing-readme-repo"))
        .expect("audit fixture");
    let report = &snapshot.missing_route_priority;

    assert!(!report.priorities.is_empty());
    assert!(report
        .boundary
        .contains("Standard audit uses no aggregate prior"));
    assert!(report
        .priorities
        .windows(2)
        .all(|pair| pair[0].priority_score_x100 >= pair[1].priority_score_x100));

    let security = report
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Security)
        .expect("security route priority");
    assert_eq!(security.calibration_estimate, None);
    assert_eq!(security.gate, GateKind::Manual);
    assert!(security
        .baseline_pattern_ids
        .contains(&"common.security.route_present".to_string()));

    let support = report
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Support)
        .expect("support route priority");
    assert_eq!(support.calibration_estimate, None);
    assert_eq!(support.gate, GateKind::Guarded);
}

#[test]
fn co_occurrence_engine_explains_combination_gaps() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let report = &snapshot.missing_route_priority;

    let automation_state = snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Automation)
        .expect("automation route state");
    assert_eq!(automation_state.state, RouteState::Structured);
    let security_state = snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Security)
        .expect("security route state");
    assert_eq!(security_state.state, RouteState::Verified);

    let supply_chain = report
        .co_occurrence_gaps
        .iter()
        .find(|gap| gap.id == "co-README-SECURITY-CI-DEPENDENCY-BOT")
        .expect("supply-chain co-occurrence gap");
    assert_eq!(supply_chain.calibration_estimate, None);
    assert_eq!(supply_chain.support_x1000, 0);
    assert_eq!(
        supply_chain.calibration_prior,
        CalibrationPriorState::NotRequested
    );
    assert!(supply_chain.present_routes.contains(&RouteKind::Identity));
    assert!(supply_chain.present_routes.contains(&RouteKind::Security));
    assert!(supply_chain.present_routes.contains(&RouteKind::Automation));
    assert!(supply_chain
        .missing_signals
        .contains(&"dependency_bot_config".to_string()));

    let security_candidate = report
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Security)
        .expect("security candidate priority");
    assert!(security_candidate.baseline_pattern_ids.is_empty());
    assert!(security_candidate
        .candidate_pattern_ids
        .contains(&"SEC-004".to_string()));
    assert_eq!(security_candidate.gate, GateKind::Guarded);

    let support_intake = report
        .co_occurrence_gaps
        .iter()
        .find(|gap| gap.id == "co-README-SUPPORT-ISSUE-FORMS")
        .expect("support intake co-occurrence gap");
    assert!(support_intake.present_routes.contains(&RouteKind::Support));
    assert!(support_intake
        .missing_signals
        .contains(&"issue_forms_yaml".to_string()));

    let ownership = report
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Ownership)
        .expect("ownership route priority");
    assert_eq!(ownership.calibration_estimate, None);
    assert!(ownership
        .co_occurrence_gap_ids
        .contains(&"co-CODEOWNERS-CI-PR-TEMPLATE".to_string()));
}

#[test]
fn q12_native_audit_json_does_not_serialize_estimates_as_observations() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("missing-readme-repo"))
        .expect("audit fixture");
    let json = seiri_report::to_json(&snapshot).expect("snapshot JSON");

    assert!(json.contains("\"calibration_estimate\""));
    assert!(!json.contains("\"estimated_repositories\""));
    for legacy_key in [
        "\"observed_gap_count\"",
        "\"observed_missing_repositories\"",
        "\"observed_missing_x1000\"",
    ] {
        assert!(
            !json.contains(legacy_key),
            "native audit JSON emitted legacy observation key {legacy_key}"
        );
    }
}

#[test]
fn q12_legacy_observation_keys_deserialize_into_typed_estimates() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("missing-readme-repo"))
        .expect("audit fixture");
    let priority = snapshot
        .missing_route_priority
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Security)
        .expect("security priority");
    let mut priority_json = serde_json::to_value(priority).expect("priority JSON");
    let priority_object = priority_json.as_object_mut().expect("priority object");
    priority_object.remove("calibration_estimate");
    priority_object.insert(
        "observed_missing_repositories".to_string(),
        serde_json::json!(558_000),
    );
    priority_object.insert("observed_missing_x1000".to_string(), serde_json::json!(558));
    let legacy_priority: MissingRoutePriority =
        serde_json::from_value(priority_json).expect("legacy priority");
    assert_eq!(
        legacy_priority
            .calibration_estimate
            .expect("legacy estimate")
            .rate_x1000,
        558
    );

    let co_snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let gap = co_snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .first()
        .expect("co-occurrence gap");
    let mut gap_json = serde_json::to_value(gap).expect("gap JSON");
    let gap_object = gap_json.as_object_mut().expect("gap object");
    let estimated_repositories = 260_000;
    gap_object.remove("calibration_estimate");
    gap_object.insert(
        "observed_repositories".to_string(),
        serde_json::json!(estimated_repositories),
    );
    let legacy_gap: RouteCoOccurrenceGap =
        serde_json::from_value(gap_json).expect("legacy co-occurrence gap");
    assert_eq!(
        legacy_gap
            .calibration_estimate
            .expect("legacy estimate")
            .estimated_repositories,
        estimated_repositories
    );
}

#[test]
fn q12_typed_estimate_rejects_inconsistent_wire_values() {
    let invalid = serde_json::json!({
        "estimated_repositories": 558_000,
        "denominator": 1_000_000,
        "rate_x1000": 999,
        "basis": "fixed_aggregate_calibration"
    });

    assert!(serde_json::from_value::<AggregateRepositoryEstimate>(invalid).is_err());
}
