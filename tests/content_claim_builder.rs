use seiri_core::{ClaimBoundaryKind, ClaimStrength, MeaningAtom, RouteKind, RouteState};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn q4_route_state_claims_are_evidence_linked() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");

    assert!(!snapshot.claims.is_empty());
    assert!(snapshot
        .claims
        .iter()
        .all(|claim| !claim.evidence_ids.is_empty()));

    let docs = snapshot
        .claims
        .iter()
        .find(|claim| {
            claim.route == RouteKind::Docs
                && claim.state == RouteState::Verified
                && claim.strength == ClaimStrength::Observed
        })
        .expect("verified docs route claim");

    assert!(docs.allowed_meanings.contains(&MeaningAtom::RouteObserved));
    assert!(docs
        .allowed_meanings
        .contains(&MeaningAtom::RouteTargetPresent));
    assert!(docs
        .boundaries
        .contains(&ClaimBoundaryKind::NotQualityGuarantee));
    assert!(docs
        .boundaries
        .contains(&ClaimBoundaryKind::NotTrustGuarantee));
}

#[test]
fn q4_missing_route_priority_can_emit_suggested_claims_with_evidence() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");

    let priority = snapshot
        .missing_route_priority
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Security)
        .expect("security priority");
    assert!(!priority.evidence_ids.is_empty());

    let suggested = snapshot
        .claims
        .iter()
        .find(|claim| {
            claim.route == RouteKind::Security && claim.strength == ClaimStrength::Suggested
        })
        .expect("security suggested claim");

    assert_eq!(suggested.state, priority.state);
    assert_eq!(suggested.evidence_ids, priority.evidence_ids);
    assert!(suggested
        .allowed_meanings
        .contains(&MeaningAtom::CalibrationCandidate));
    assert!(suggested
        .boundaries
        .contains(&ClaimBoundaryKind::NotAutomaticWeightAdoption));
}

#[test]
fn q4_builder_skips_claims_without_evidence() {
    let snapshot =
        seiri_report::audit_repository(fixture("missing-readme-repo")).expect("audit fixture");

    let security_state = snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Security)
        .expect("security route state");
    assert_eq!(security_state.state, RouteState::UnsafeToInvent);
    assert!(security_state.evidence_ids.is_empty());

    assert!(!snapshot.claims.iter().any(|claim| {
        claim.route == RouteKind::Security && claim.state == RouteState::UnsafeToInvent
    }));
    assert!(snapshot
        .claims
        .iter()
        .all(|claim| !claim.evidence_ids.is_empty()));
}

#[test]
fn q4_json_and_markdown_expose_content_claims() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");

    let json = seiri_report::to_json(&snapshot).expect("render JSON");
    let parsed = serde_json::from_str::<Value>(&json).expect("valid JSON");
    let claims = parsed["claims"].as_array().expect("claims array");
    assert!(!claims.is_empty());
    assert!(claims.iter().all(|claim| claim["evidence_ids"]
        .as_array()
        .is_some_and(|ids| !ids.is_empty())));

    let markdown = seiri_report::to_markdown(&snapshot);
    assert!(markdown.contains("- Content claims: `"));
    assert!(markdown.contains("## Content Claims"));
    assert!(markdown.contains("`claim-0001`"));
    assert!(markdown.contains("Boundaries:"));
}

#[test]
fn q5_markdown_report_binds_routes_and_priorities_to_claims() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");

    let markdown = seiri_report::to_markdown(&snapshot);
    assert!(markdown.contains("- Summary: total `"));
    assert!(markdown.contains("claims `claim-"));
    assert!(markdown.contains("Claim IDs: `claim-"));
    assert!(markdown.contains("Boundary kinds: `Not"));
}
