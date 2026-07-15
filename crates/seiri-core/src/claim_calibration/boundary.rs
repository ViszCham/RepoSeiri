use super::{ClaimBoundaryMask, MeaningMask};
use crate::{ClaimBoundaryKind, ClaimStrength, MeaningAtom, RouteKind, RouteState};

#[must_use]
pub fn resolve_claim_boundaries(
    route: RouteKind,
    state: RouteState,
    strength: ClaimStrength,
    meanings: MeaningMask,
) -> ClaimBoundaryMask {
    let mut mask = ClaimBoundaryMask::default();
    for boundary in route_claim_boundaries(route) {
        mask = mask.with(*boundary);
    }

    if state == RouteState::Stale {
        mask = mask.with(ClaimBoundaryKind::NotMaintenanceGuarantee);
    }
    if matches!(state, RouteState::Conflicting | RouteState::UnsafeToInvent) {
        mask = mask
            .with(ClaimBoundaryKind::NotOwnerApproval)
            .with(ClaimBoundaryKind::NotAutomaticPolicyAdoption);
    }
    if strength == ClaimStrength::Suggested || meanings.contains(MeaningAtom::CalibrationCandidate)
    {
        mask = mask
            .with(ClaimBoundaryKind::NotAutomaticPolicyAdoption)
            .with(ClaimBoundaryKind::NotAutomaticWeightAdoption);
    }
    if strength == ClaimStrength::Blocked || meanings.contains(MeaningAtom::PatchPreviewOnly) {
        mask = mask
            .with(ClaimBoundaryKind::NotOwnerApproval)
            .with(ClaimBoundaryKind::NotAutomaticPolicyAdoption);
    }
    if meanings.contains(MeaningAtom::AutomationConfigured)
        || meanings.contains(MeaningAtom::ExpectedOutputDocumented)
    {
        mask = mask.with(ClaimBoundaryKind::NotRuntimeVerification);
    }

    mask
}

#[must_use]
pub const fn route_claim_boundaries(route: RouteKind) -> &'static [ClaimBoundaryKind] {
    match route {
        RouteKind::Identity => &[
            ClaimBoundaryKind::NotPopularityGuarantee,
            ClaimBoundaryKind::NotTrustGuarantee,
        ],
        RouteKind::Docs => &[ClaimBoundaryKind::NotQualityGuarantee],
        RouteKind::Quickstart => &[
            ClaimBoundaryKind::NotRuntimeVerification,
            ClaimBoundaryKind::NotProductionReadiness,
        ],
        RouteKind::Support => &[ClaimBoundaryKind::NotMaintenanceGuarantee],
        RouteKind::Intake | RouteKind::Contributing | RouteKind::Governance => &[
            ClaimBoundaryKind::NotOwnerApproval,
            ClaimBoundaryKind::NotAutomaticPolicyAdoption,
        ],
        RouteKind::Security => &[
            ClaimBoundaryKind::NotSecurityGuarantee,
            ClaimBoundaryKind::NotProductionReadiness,
        ],
        RouteKind::Release => &[
            ClaimBoundaryKind::NotPublicationReadiness,
            ClaimBoundaryKind::NotMaintenanceGuarantee,
        ],
        RouteKind::Lifecycle => &[
            ClaimBoundaryKind::NotMaintenanceGuarantee,
            ClaimBoundaryKind::NotAutomaticPolicyAdoption,
        ],
        RouteKind::License => &[
            ClaimBoundaryKind::NotLegalFitnessGuarantee,
            ClaimBoundaryKind::NotLegalAdvice,
        ],
        RouteKind::Automation => &[
            ClaimBoundaryKind::NotRuntimeVerification,
            ClaimBoundaryKind::NotSecurityGuarantee,
        ],
        RouteKind::Ownership => &[ClaimBoundaryKind::NotOwnerApproval],
        RouteKind::Hygiene => &[
            ClaimBoundaryKind::NotQualityGuarantee,
            ClaimBoundaryKind::NotSecurityGuarantee,
        ],
        RouteKind::Unknown => &[
            ClaimBoundaryKind::NotPopularityGuarantee,
            ClaimBoundaryKind::NotTrustGuarantee,
            ClaimBoundaryKind::NotSecurityGuarantee,
            ClaimBoundaryKind::NotQualityGuarantee,
            ClaimBoundaryKind::NotLegalFitnessGuarantee,
            ClaimBoundaryKind::NotLegalAdvice,
            ClaimBoundaryKind::NotMaintenanceGuarantee,
            ClaimBoundaryKind::NotRuntimeVerification,
            ClaimBoundaryKind::NotPublicationReadiness,
            ClaimBoundaryKind::NotOwnerApproval,
            ClaimBoundaryKind::NotProductionReadiness,
            ClaimBoundaryKind::NotAutomaticPolicyAdoption,
            ClaimBoundaryKind::NotAutomaticWeightAdoption,
        ],
    }
}
