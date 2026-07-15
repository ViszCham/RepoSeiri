use seiri_core::{
    calibrate_content_claim, evaluate_underclaim_loss, AggregatePrior, CalibrationKey,
    CalibrationLookup, CalibrationProvider, ClaimAssertionKind, ClaimBoundaryKind,
    ClaimProjectionCandidate, ClaimStrength, PriorBasis, PriorVisibility, ProjectedAssertionLevel,
    RouteKind, UnderclaimCauseMask,
};
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn observed_claims_project_positive_first_at_evidence_strength() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");

    let observed = snapshot
        .claims
        .iter()
        .filter(|claim| claim.strength == ClaimStrength::Observed)
        .collect::<Vec<_>>();
    assert!(!observed.is_empty());
    for claim in observed {
        let projection = calibrate_content_claim(claim);
        assert!(projection.assertion.kind.is_positive());
        assert_eq!(projection.underclaim_loss.score_x100(), 0);
        assert!(!projection.assertion.render_sentence().is_empty());
        assert!(!claim.evidence_ids.is_empty());
    }
}

#[test]
fn claim_local_boundaries_are_relevance_scoped() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let docs = snapshot
        .claims
        .iter()
        .find(|claim| claim.route == RouteKind::Docs && claim.strength == ClaimStrength::Observed)
        .expect("observed docs claim");
    let docs_projection = calibrate_content_claim(docs);
    assert!(docs_projection
        .boundaries
        .contains(ClaimBoundaryKind::NotQualityGuarantee));
    assert!(!docs_projection
        .boundaries
        .contains(ClaimBoundaryKind::NotPopularityGuarantee));
    assert!(!docs_projection
        .boundaries
        .contains(ClaimBoundaryKind::NotTrustGuarantee));

    let suggested = snapshot
        .claims
        .iter()
        .find(|claim| claim.strength == ClaimStrength::Suggested)
        .expect("suggested claim");
    let suggested_projection = calibrate_content_claim(suggested);
    assert_eq!(
        suggested_projection.assertion.kind,
        ClaimAssertionKind::ReviewCandidate
    );
    assert!(suggested_projection
        .boundaries
        .contains(ClaimBoundaryKind::NotAutomaticPolicyAdoption));
    assert!(suggested_projection
        .boundaries
        .contains(ClaimBoundaryKind::NotAutomaticWeightAdoption));
}

#[test]
fn boundary_only_wording_has_underclaim_loss() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let claim = snapshot
        .claims
        .iter()
        .find(|claim| claim.strength == ClaimStrength::Observed)
        .expect("observed claim");
    let loss = evaluate_underclaim_loss(
        claim,
        ClaimProjectionCandidate {
            assertion_level: ProjectedAssertionLevel::BoundaryOnly,
            evidence_linked: false,
            positive_first: false,
        },
    );

    assert_eq!(loss.score_x100(), 100);
    assert!(loss
        .causes()
        .contains(UnderclaimCauseMask::MISSING_POSITIVE_ASSERTION));
    assert!(loss
        .causes()
        .contains(UnderclaimCauseMask::OBSERVED_DOWNGRADED));
    assert!(loss
        .causes()
        .contains(UnderclaimCauseMask::EVIDENCE_OMITTED));
    assert!(loss.causes().contains(UnderclaimCauseMask::BOUNDARY_FIRST));
}

#[test]
fn canonical_markdown_and_codex_governance_are_positive_first() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let markdown = seiri_report::to_markdown(&snapshot);
    let first_claim_line = markdown
        .lines()
        .find(|line| line.starts_with("- `claim-"))
        .expect("claim line");
    assert!(first_claim_line.contains("The audit observed"));
    assert!(first_claim_line.find("The audit observed") < first_claim_line.find("Boundaries:"));

    let plan = seiri_planner::plan_patches(&snapshot);
    let view = seiri_codex::CodexView::new(&snapshot, &plan, None);
    let governance =
        seiri_codex::render_query_markdown(&view.query(seiri_codex::CodexQueryKind::Governance));
    assert!(governance.contains("## Evidence-Backed Claims"));
    assert!(governance.contains("The audit observed"));
    assert_eq!(governance.matches("- Boundary:").count(), 1);
}

#[test]
fn content_claim_wire_shape_remains_v2_compatible() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let claim = snapshot.claims.first().expect("claim");
    let value = serde_json::to_value(claim).expect("claim JSON");
    let mut keys = value
        .as_object()
        .expect("claim object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "allowed_meanings",
            "boundaries",
            "evidence_ids",
            "id",
            "route",
            "state",
            "strength",
        ]
    );
}

#[test]
fn private_calibration_cannot_weaken_observed_claims() {
    let root = fixture("readme-route-repo");
    let standard =
        seiri_report::audit_repository_with_profile(&root, seiri_core::ProfileKind::Common)
            .expect("standard audit");
    let calibrated = seiri_report::audit_repository_with_calibration_provider(
        &root,
        seiri_core::ProfileKind::Common,
        &SyntheticPrivateCalibration,
    )
    .expect("calibrated audit");

    let observed_projection = |snapshot: &seiri_core::RepositoryAnalysis| {
        snapshot
            .claims
            .iter()
            .filter(|claim| claim.strength == ClaimStrength::Observed)
            .map(|claim| {
                let projection = calibrate_content_claim(claim);
                (
                    claim.route,
                    claim.state,
                    projection.assertion.kind,
                    projection.underclaim_loss.score_x100(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        observed_projection(&standard),
        observed_projection(&calibrated)
    );

    let public = [
        seiri_report::to_json(&calibrated).expect("analysis JSON"),
        seiri_report::to_markdown(&calibrated),
    ];
    for surface in public {
        assert!(!surface.contains("987654"));
        assert!(!surface.contains("PRIVATE_ASSERTION_CALIBRATION"));
    }
}

struct SyntheticPrivateCalibration;

impl CalibrationProvider for SyntheticPrivateCalibration {
    fn prior(&self, key: &CalibrationKey) -> CalibrationLookup {
        match key {
            CalibrationKey::RouteGap(_) | CalibrationKey::ProfileBranch(_) => {
                CalibrationLookup::Available(
                    AggregatePrior::try_new(
                        987_654,
                        NonZeroU64::new(1_000_000).expect("nonzero sample"),
                        73,
                        PriorBasis::AggregateAnalysis,
                    )
                    .expect("valid synthetic prior"),
                )
            }
            CalibrationKey::CoOccurrence(_) => CalibrationLookup::NotRequested,
        }
    }

    fn visibility(&self) -> Option<PriorVisibility> {
        Some(PriorVisibility::LocalOnly)
    }

    fn redacted_fingerprint(&self) -> Option<&str> {
        Some("synthetic-private-assertion-calibration")
    }
}
