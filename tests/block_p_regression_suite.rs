use seiri_core::{
    EvidenceConfidence, GateKind, ProfileKind, ProfilePriority, RouteKind, RouteState,
    SCHEMA_VERSION,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn block_p_profile_fixture_matrix_locks_9_profile_branch_cases() {
    let branch_profiles = seiri_profiles::branch_profiles()
        .iter()
        .map(|(profile, _)| *profile)
        .collect::<BTreeSet<_>>();
    let expected_profiles = [
        ProfileKind::Library,
        ProfileKind::Infra,
        ProfileKind::Cli,
        ProfileKind::Product,
        ProfileKind::Runtime,
        ProfileKind::Docs,
        ProfileKind::Tutorial,
        ProfileKind::Ml,
        ProfileKind::Template,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(branch_profiles, expected_profiles);

    let cases = [
        ("profile-library-regression", ProfileKind::Library, 93),
        ("profile-cli-regression", ProfileKind::Cli, 88),
        ("profile-infra-regression", ProfileKind::Infra, 85),
        ("profile-product-regression", ProfileKind::Product, 85),
        ("profile-runtime-regression", ProfileKind::Runtime, 82),
        ("profile-docs-regression", ProfileKind::Docs, 77),
        ("profile-tutorial-regression", ProfileKind::Tutorial, 84),
        ("profile-ml-regression", ProfileKind::Ml, 82),
        ("profile-template-regression", ProfileKind::Template, 75),
    ];

    for (fixture_name, expected_profile, expected_confidence) in cases {
        let snapshot =
            seiri_report::audit_repository_with_profile(fixture(fixture_name), ProfileKind::Common)
                .expect("audit profile fixture");
        assert_eq!(snapshot.schema_version, SCHEMA_VERSION);
        let profile = snapshot.profile.as_ref().expect("profile report");
        assert_eq!(profile.branch_summary.emitted_profiles, 9);
        assert_eq!(profile.branches.len(), 9);
        let top = profile.branches.first().expect("top profile branch");
        assert_eq!(
            top.profile, expected_profile,
            "fixture {fixture_name} top profile changed"
        );
        assert_eq!(
            top.confidence_x100, expected_confidence,
            "fixture {fixture_name} confidence changed"
        );
        assert_eq!(profile.branch_summary.top_profile, Some(expected_profile));
        assert_eq!(
            profile.branch_summary.top_confidence_x100,
            Some(expected_confidence)
        );
        assert_eq!(top.evidence_score_x100, 100);
        assert!(!top.matched_signals.is_empty());
        assert!(top.rationale.contains("not a repository type assertion"));
    }
}

#[test]
fn block_p_route_state_matrix_locks_representative_states() {
    let cases = [
        (
            "missing-readme-repo",
            RouteKind::License,
            RouteState::Structured,
            EvidenceConfidence::High,
        ),
        (
            "missing-readme-repo",
            RouteKind::Security,
            RouteState::UnsafeToInvent,
            EvidenceConfidence::Medium,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::Identity,
            RouteState::Verified,
            EvidenceConfidence::High,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::Docs,
            RouteState::Conflicting,
            EvidenceConfidence::Medium,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::Support,
            RouteState::Weak,
            EvidenceConfidence::Low,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::Security,
            RouteState::Stale,
            EvidenceConfidence::Medium,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::Release,
            RouteState::Overloaded,
            EvidenceConfidence::Medium,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::Governance,
            RouteState::Routed,
            EvidenceConfidence::Medium,
        ),
        (
            "safe-plan-repo",
            RouteKind::Support,
            RouteState::Absent,
            EvidenceConfidence::Low,
        ),
        (
            "nested-license-only-repo",
            RouteKind::License,
            RouteState::Inherited,
            EvidenceConfidence::Low,
        ),
    ];

    let mut seen_states = BTreeSet::new();
    for (fixture_name, route, expected_state, expected_confidence) in cases {
        let snapshot =
            seiri_report::audit_repository(fixture(fixture_name)).expect("audit fixture");
        let route_state = snapshot
            .route_states
            .iter()
            .find(|state| state.route == route)
            .expect("route state");
        assert_eq!(
            route_state.state, expected_state,
            "fixture {fixture_name} route {route:?} state changed"
        );
        assert_eq!(
            route_state.confidence, expected_confidence,
            "fixture {fixture_name} route {route:?} confidence changed"
        );
        seen_states.insert(expected_state);
    }

    assert_eq!(seen_states.len(), 10);
}

#[test]
fn block_p_gate_and_co_occurrence_regression_matrix_is_stable() {
    let priority_cases = [
        (
            "missing-readme-repo",
            RouteKind::Identity,
            GateKind::Manual,
            ProfilePriority::Critical,
            100,
        ),
        (
            "readme-route-map-v2-repo",
            RouteKind::License,
            GateKind::Manual,
            ProfilePriority::Critical,
            100,
        ),
        (
            "safe-plan-repo",
            RouteKind::Security,
            GateKind::Manual,
            ProfilePriority::Critical,
            100,
        ),
        (
            "security-support-intake-automation-repo",
            RouteKind::Docs,
            GateKind::Safe,
            ProfilePriority::High,
            71,
        ),
        (
            "security-support-intake-automation-repo",
            RouteKind::Release,
            GateKind::Guarded,
            ProfilePriority::Critical,
            86,
        ),
        (
            "nested-license-only-repo",
            RouteKind::License,
            GateKind::Manual,
            ProfilePriority::Critical,
            100,
        ),
    ];

    for (fixture_name, route, expected_gate, expected_priority, expected_score) in priority_cases {
        let snapshot =
            seiri_report::audit_repository(fixture(fixture_name)).expect("audit fixture");
        let priority = snapshot
            .missing_route_priority
            .priorities
            .iter()
            .find(|priority| priority.route == route)
            .expect("route priority");
        assert_eq!(
            priority.gate, expected_gate,
            "fixture {fixture_name} route {route:?} gate changed"
        );
        assert_eq!(priority.priority, expected_priority);
        assert_eq!(priority.priority_score_x100, expected_score);
    }

    let co_occurrence_cases = [
        (
            "missing-readme-repo",
            "co-README-LICENSE",
            GateKind::Manual,
            ProfilePriority::Critical,
            905,
        ),
        (
            "readme-route-map-v2-repo",
            "co-README-SUPPORT-ISSUE-FORMS",
            GateKind::Guarded,
            ProfilePriority::High,
            300,
        ),
        (
            "safe-plan-repo",
            "co-README-SECURITY-CI-DEPENDENCY-BOT",
            GateKind::Guarded,
            ProfilePriority::Normal,
            260,
        ),
        (
            "security-support-intake-automation-repo",
            "co-CODEOWNERS-CI-PR-TEMPLATE",
            GateKind::Manual,
            ProfilePriority::Normal,
            240,
        ),
    ];

    for (fixture_name, gap_id, expected_gate, expected_priority, expected_support) in
        co_occurrence_cases
    {
        let snapshot =
            seiri_report::audit_repository(fixture(fixture_name)).expect("audit fixture");
        let gap = snapshot
            .missing_route_priority
            .co_occurrence_gaps
            .iter()
            .find(|gap| gap.id == gap_id)
            .expect("co-occurrence gap");
        assert_eq!(
            gap.gate, expected_gate,
            "fixture {fixture_name} gap {gap_id}"
        );
        assert_eq!(gap.priority, expected_priority);
        assert_eq!(gap.support_x1000, expected_support);
        assert!(!gap.reason.is_empty());
    }
}
