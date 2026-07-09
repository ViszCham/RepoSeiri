use seiri_core::{
    ClaimBoundaryKind, ClaimStrength, MeaningAtom, RouteKind, RouteState, WordingRuleKind,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn q11_lifecycle_and_verified_security_routes_keep_claim_boundaries() {
    let lifecycle_readme = seiri_markdown::analyze_readme(fixture("lifecycle-route-repo"))
        .expect("read lifecycle README")
        .expect("lifecycle README exists");
    let lifecycle_readme_route = lifecycle_readme
        .route_map
        .entries
        .iter()
        .find(|entry| entry.route == RouteKind::Lifecycle)
        .expect("lifecycle README route entry");
    assert_eq!(lifecycle_readme_route.state, RouteState::Verified);

    let lifecycle_snapshot = seiri_report::audit_repository(fixture("lifecycle-route-repo"))
        .expect("audit lifecycle fixture");
    let lifecycle = lifecycle_snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Lifecycle)
        .expect("lifecycle route state");
    assert_eq!(lifecycle.state, RouteState::Routed);
    assert!(!lifecycle.evidence_ids.is_empty());

    let security_snapshot = seiri_report::audit_repository(fixture("verified-security-route-repo"))
        .expect("audit verified security fixture");
    let security = security_snapshot
        .route_states
        .iter()
        .find(|state| state.route == RouteKind::Security)
        .expect("security route state");
    assert_eq!(security.state, RouteState::Verified);
    assert!(!security.evidence_ids.is_empty());

    let claim = security_snapshot
        .claims
        .iter()
        .find(|claim| {
            claim.route == RouteKind::Security
                && claim.state == RouteState::Verified
                && claim.strength == ClaimStrength::Observed
        })
        .expect("verified security route claim");
    assert!(!claim.evidence_ids.is_empty());
    assert!(claim.allowed_meanings.contains(&MeaningAtom::RouteObserved));
    assert!(claim
        .allowed_meanings
        .contains(&MeaningAtom::RouteTargetPresent));
    assert!(!claim
        .allowed_meanings
        .contains(&MeaningAtom::HumanReviewRequired));
    assert!(claim
        .boundaries
        .contains(&ClaimBoundaryKind::NotSecurityGuarantee));
    assert!(claim
        .boundaries
        .contains(&ClaimBoundaryKind::NotQualityGuarantee));
    assert!(claim
        .boundaries
        .contains(&ClaimBoundaryKind::NotTrustGuarantee));

    let rule = seiri_core::route_meaning_rule(RouteKind::Security, RouteState::Verified);
    assert!(rule.indicates.contains(&MeaningAtom::RouteTargetPresent));
    assert!(rule
        .does_not_indicate
        .contains(&ClaimBoundaryKind::NotSecurityGuarantee));
    assert!(rule
        .does_not_indicate
        .contains(&ClaimBoundaryKind::NotQualityGuarantee));

    let report_markdown = seiri_report::to_markdown(&security_snapshot);
    assert!(report_markdown.contains("## Content Claims"));
    assert!(!report_markdown.contains("guarantees security"));
    assert!(!report_markdown.contains("security guarantee"));
}

#[test]
fn q11_wording_lint_locks_positive_and_negative_fixtures() {
    let positive = seiri_report::lint_wording_repository(fixture("wording-lint-repo"))
        .expect("positive wording report");
    assert_eq!(positive.summary.findings, 4);
    assert!(positive
        .findings
        .iter()
        .any(|finding| finding.rule == WordingRuleKind::SecurityGuarantee));
    assert!(positive
        .findings
        .iter()
        .any(|finding| finding.rule == WordingRuleKind::ProductionReadiness));

    let negative = seiri_report::lint_wording_repository(fixture("wording-safe-repo"))
        .expect("negative wording report");
    assert_eq!(negative.summary.files_scanned, 1);
    assert_eq!(negative.summary.generated_surfaces, 3);
    assert_eq!(negative.summary.findings, 0);
    assert!(
        negative.summary.suppressed_boundary_exceptions >= 4,
        "safe fixture should exercise negated and typed boundary exceptions"
    );

    let markdown = seiri_report::wording_lint_to_markdown(&negative);
    assert!(markdown.contains("- No overclaim wording findings emitted."));
}
