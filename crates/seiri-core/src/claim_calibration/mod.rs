mod boundary;
mod mask;
mod projection;

pub use boundary::{resolve_claim_boundaries, route_claim_boundaries};
pub use mask::{ClaimBoundaryMask, MeaningMask};
pub use projection::{
    calibrate_content_claim, evaluate_projection_admissibility, evaluate_underclaim_loss,
    project_content_claim, CalibratedClaimProjection, ClaimAssertion, ClaimAssertionKind,
    ClaimEvidencePosture, ClaimProjectionCandidate, ContentClaimProjection,
    ProjectedAssertionLevel, ProjectionAdmissibility, ProjectionRejectMask, ProjectionRejectReason,
    UnderclaimCauseMask, UnderclaimLoss, CLAIM_SEMANTIC_REVISION,
};
