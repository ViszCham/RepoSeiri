mod boundary;
mod mask;
mod projection;

pub use boundary::{resolve_claim_boundaries, route_claim_boundaries};
pub use mask::{ClaimBoundaryMask, MeaningMask};
pub use projection::{
    calibrate_content_claim, evaluate_underclaim_loss, CalibratedClaimProjection, ClaimAssertion,
    ClaimAssertionKind, ClaimEvidencePosture, ClaimProjectionCandidate, ProjectedAssertionLevel,
    UnderclaimCauseMask, UnderclaimLoss,
};
