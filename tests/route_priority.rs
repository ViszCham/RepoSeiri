use seiri_core::{GateKind, ProfilePriority, RouteKind};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn missing_route_priority_uses_fixed_analysis_gap_priors() {
    let snapshot =
        seiri_report::audit_repository(fixture("missing-readme-repo")).expect("audit fixture");
    let report = &snapshot.missing_route_priority;

    assert!(!report.priorities.is_empty());
    assert!(report.boundary.contains("fixed 1,000,000-repository"));
    assert!(report
        .priorities
        .windows(2)
        .all(|pair| pair[0].priority_score_x100 >= pair[1].priority_score_x100));

    let security = report
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Security)
        .expect("security route priority");
    assert_eq!(security.observed_missing_repositories, Some(558_000));
    assert_eq!(security.observed_missing_x1000, Some(558));
    assert_eq!(security.gate, GateKind::Manual);
    assert_eq!(security.priority, ProfilePriority::Critical);
    assert!(security
        .baseline_pattern_ids
        .contains(&"common.security.route_present".to_string()));

    let support = report
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Support)
        .expect("support route priority");
    assert_eq!(support.observed_missing_repositories, Some(503_000));
    assert_eq!(support.gate, GateKind::Guarded);
}

#[test]
fn co_occurrence_engine_explains_combination_gaps() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");
    let report = &snapshot.missing_route_priority;

    let supply_chain = report
        .co_occurrence_gaps
        .iter()
        .find(|gap| gap.id == "co-README-SECURITY-CI-DEPENDENCY-BOT")
        .expect("supply-chain co-occurrence gap");
    assert_eq!(supply_chain.observed_repositories, 260_000);
    assert_eq!(supply_chain.support_x1000, 260);
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
    assert_eq!(ownership.observed_missing_repositories, Some(605_000));
    assert!(ownership
        .co_occurrence_gap_ids
        .contains(&"co-CODEOWNERS-CI-PR-TEMPLATE".to_string()));
}
