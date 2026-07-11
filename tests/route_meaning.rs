use seiri_core::{
    route_meaning_rule, route_meaning_rules, route_state_does_not_indicate, route_state_indicates,
    ClaimBoundaryKind, MeaningAtom, RouteKind, RouteState, ROUTE_MEANING_ROUTES,
    ROUTE_MEANING_STATES,
};

#[test]
fn registry_covers_every_route_and_major_state_pair() {
    let rules = route_meaning_rules().collect::<Vec<_>>();
    assert_eq!(
        rules.len(),
        ROUTE_MEANING_ROUTES.len() * ROUTE_MEANING_STATES.len()
    );

    for route in ROUTE_MEANING_ROUTES {
        for state in ROUTE_MEANING_STATES {
            let rule = route_meaning_rule(*route, *state);
            assert_eq!(rule.route, *route);
            assert_eq!(rule.state, *state);
            assert!(!rule.indicates.is_empty());
            assert!(!rule.does_not_indicate.is_empty());
            assert!(rules.contains(&rule));
        }
    }
}

#[test]
fn verified_security_route_does_not_become_a_guarantee() {
    let rule = route_meaning_rule(RouteKind::Security, RouteState::Verified);
    assert!(rule.indicates.contains(&MeaningAtom::RouteObserved));
    assert!(rule
        .indicates
        .contains(&MeaningAtom::RepositoryLocalTargetPresent));

    for boundary in [
        ClaimBoundaryKind::NotPopularityGuarantee,
        ClaimBoundaryKind::NotTrustGuarantee,
        ClaimBoundaryKind::NotSecurityGuarantee,
        ClaimBoundaryKind::NotQualityGuarantee,
        ClaimBoundaryKind::NotLegalFitnessGuarantee,
        ClaimBoundaryKind::NotMaintenanceGuarantee,
        ClaimBoundaryKind::NotRuntimeVerification,
        ClaimBoundaryKind::NotPublicationReadiness,
    ] {
        assert!(
            rule.does_not_indicate.contains(&boundary),
            "missing boundary: {boundary:?}"
        );
    }
}

#[test]
fn state_meanings_keep_missing_weak_and_verified_separate() {
    assert_eq!(
        route_state_indicates(RouteState::Absent),
        &[MeaningAtom::RouteMissing]
    );
    assert!(route_state_indicates(RouteState::Weak).contains(&MeaningAtom::HumanReviewRequired));
    assert!(route_state_indicates(RouteState::Verified)
        .contains(&MeaningAtom::RepositoryLocalTargetPresent));
    assert!(
        !route_state_indicates(RouteState::Verified).contains(&MeaningAtom::HumanReviewRequired)
    );
    assert!(
        route_state_indicates(RouteState::UnsafeToInvent).contains(&MeaningAtom::PatchPreviewOnly)
    );
}

#[test]
fn route_meaning_rule_serializes_to_stable_json_surface() {
    let rule = route_meaning_rule(RouteKind::Docs, RouteState::Structured);
    let json = serde_json::to_value(rule).expect("serialize route meaning rule");

    assert_eq!(json["route"], "docs");
    assert_eq!(json["state"], "structured");
    assert_eq!(json["indicates"][0], "structured_file_present");
    assert_eq!(json["does_not_indicate"][0], "not_popularity_guarantee");
}

#[test]
fn non_claim_boundaries_are_available_from_route_state_pair() {
    let boundaries = route_state_does_not_indicate(RouteKind::License, RouteState::Verified);
    assert!(boundaries.contains(&ClaimBoundaryKind::NotLegalAdvice));
    assert!(boundaries.contains(&ClaimBoundaryKind::NotAutomaticPolicyAdoption));
    assert!(boundaries.contains(&ClaimBoundaryKind::NotAutomaticWeightAdoption));
}

#[test]
fn route_target_meaning_rejects_removed_name() {
    assert!(serde_json::from_str::<MeaningAtom>("\"route_target_present\"").is_err());
    let atom = MeaningAtom::RepositoryLocalTargetPresent;
    assert_eq!(
        serde_json::to_string(&atom).expect("canonical meaning atom"),
        "\"repository_local_target_present\""
    );
}
