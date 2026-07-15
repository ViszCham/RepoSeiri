use seiri_core::{
    stable_claim_id, stable_evidence_id, ClaimBoundary, ClaimBoundaryKind, ClaimRefIndex,
    ClaimStrength, ContentClaim, MeaningAtom, RouteKind, RouteState,
};
use serde_json::{json, Value};

#[test]
fn content_claim_core_schema_roundtrips_through_json() {
    let claim = ContentClaim::new(
        7,
        RouteKind::Security,
        RouteState::Verified,
        ClaimStrength::Observed,
        vec![stable_evidence_id(1), stable_evidence_id(2)],
        vec![
            MeaningAtom::RouteObserved,
            MeaningAtom::StructuredFilePresent,
        ],
    );

    assert_eq!(claim.id(), "claim-0007");
    assert_eq!(claim.id(), &stable_claim_id(7));
    assert_eq!(claim.evidence_ids().len(), 2);
    assert!(claim
        .boundaries()
        .contains(&ClaimBoundaryKind::NotSecurityGuarantee));

    let json = serde_json::to_string(&claim).expect("serialize content claim");
    let parsed = serde_json::from_str::<Value>(&json).expect("parse content claim json");
    assert_eq!(parsed["id"], "claim-0007");
    assert_eq!(parsed["route"], "security");
    assert_eq!(parsed["state"], "verified");
    assert_eq!(parsed["strength"], "observed");
    assert_eq!(parsed["allowed_meanings"][0], "route_observed");
    assert_eq!(parsed["boundaries"][0], "not_security_guarantee");

    let roundtrip = serde_json::from_str::<ContentClaim>(&json).expect("deserialize content claim");
    assert_eq!(roundtrip, claim);
}

#[test]
fn content_claim_rejects_noncanonical_wire_data() {
    let tampered = json!({
        "id": "claim-0001",
        "route": "security",
        "state": "verified",
        "strength": "observed",
        "evidence_ids": [],
        "allowed_meanings": ["route_observed"],
        "boundaries": ["not_popularity_guarantee"]
    });
    let error = serde_json::from_value::<ContentClaim>(tampered).expect_err("must reject");
    assert!(error
        .to_string()
        .contains("observed content claim requires evidence"));

    let mut value = serde_json::to_value(ContentClaim::new(
        1,
        RouteKind::Security,
        RouteState::Verified,
        ClaimStrength::Observed,
        vec![stable_evidence_id(1)],
        vec![MeaningAtom::RouteObserved],
    ))
    .unwrap();
    value["boundaries"] = json!(["not_popularity_guarantee"]);
    let error = serde_json::from_value::<ContentClaim>(value).expect_err("must reject");
    assert!(error
        .to_string()
        .contains("boundaries do not match canonical semantics"));
}

#[test]
fn claim_boundary_keeps_canonical_review_surface() {
    let boundary = ClaimBoundary {
        summary: "Calibration output is candidate evidence for human review.".to_string(),
        review_required: true,
        runtime_rule_adoption_allowed: false,
        automatic_weight_adoption_allowed: false,
        guarantee_allowed: false,
        blocked_claims: vec![
            "popularity guarantee".to_string(),
            "trust guarantee".to_string(),
            "security guarantee".to_string(),
            "quality guarantee".to_string(),
        ],
    };

    let json = serde_json::to_value(&boundary).expect("serialize claim boundary");
    assert_eq!(json["review_required"], true);
    assert_eq!(json["runtime_rule_adoption_allowed"], false);
    assert_eq!(json["automatic_weight_adoption_allowed"], false);
    assert_eq!(json["guarantee_allowed"], false);
    assert_eq!(json["blocked_claims"][0], "popularity guarantee");
}

#[test]
fn claim_boundary_kind_covers_required_public_claim_blocks() {
    let required = [
        ClaimBoundaryKind::NotPopularityGuarantee,
        ClaimBoundaryKind::NotTrustGuarantee,
        ClaimBoundaryKind::NotSecurityGuarantee,
        ClaimBoundaryKind::NotQualityGuarantee,
        ClaimBoundaryKind::NotLegalFitnessGuarantee,
        ClaimBoundaryKind::NotMaintenanceGuarantee,
        ClaimBoundaryKind::NotRuntimeVerification,
        ClaimBoundaryKind::NotPublicationReadiness,
    ];

    let json = serde_json::to_value(required).expect("serialize claim boundary kinds");
    assert_eq!(json[0], "not_popularity_guarantee");
    assert_eq!(json[4], "not_legal_fitness_guarantee");
    assert_eq!(json[7], "not_publication_readiness");
}

#[test]
fn claim_ref_index_groups_ids_and_deduplicates_boundaries() {
    let claims = vec![
        ContentClaim::new(
            1,
            RouteKind::Security,
            RouteState::Verified,
            ClaimStrength::Observed,
            vec![stable_evidence_id(1)],
            vec![MeaningAtom::RouteObserved],
        ),
        ContentClaim::new(
            2,
            RouteKind::Security,
            RouteState::Verified,
            ClaimStrength::Suggested,
            vec![stable_evidence_id(2)],
            vec![MeaningAtom::CalibrationCandidate],
        ),
    ];
    let index = ClaimRefIndex::new(&claims);

    assert_eq!(
        index.claim_ids_for_route_state(RouteKind::Security, RouteState::Verified),
        vec!["claim-0001".to_string(), "claim-0002".to_string()]
    );
    assert_eq!(index.strength_count(ClaimStrength::Suggested), 1);
    let boundaries = index.boundary_kinds_for_route(RouteKind::Security);
    assert_eq!(boundaries.len(), 4);
    assert!(boundaries.contains(&ClaimBoundaryKind::NotSecurityGuarantee));
    assert!(boundaries.contains(&ClaimBoundaryKind::NotProductionReadiness));
    assert!(boundaries.contains(&ClaimBoundaryKind::NotAutomaticWeightAdoption));
    assert!(boundaries.contains(&ClaimBoundaryKind::NotAutomaticPolicyAdoption));
}
