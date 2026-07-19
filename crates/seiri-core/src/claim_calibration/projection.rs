use super::{resolve_claim_boundaries, ClaimBoundaryMask, MeaningMask};
use crate::{
    ClaimBoundaryKind, ClaimId, ClaimStrength, ContentClaim, EvidenceId, MeaningAtom, RouteKind,
    RouteState,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimEvidencePosture {
    DirectObservation,
    BoundedInference,
    ReviewSuggestion,
    BlockedForHumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimAssertionKind {
    RepositoryLocalTargetObserved,
    StructuredRouteObserved,
    ReadmeRouteObserved,
    RouteGapObserved,
    EvidenceScopedRouteState,
    ReviewCandidate,
    HeldForHumanReview,
}

impl ClaimAssertionKind {
    #[must_use]
    pub const fn has_primary_assertion(self) -> bool {
        !matches!(self, Self::HeldForHumanReview)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionRejectReason {
    ObservedWithoutEvidence,
    VerifiedWithoutLocalTargetMeaning,
    PositiveAssertionForBlockedClaim,
}

const ALL_REJECTION_REASONS: [ProjectionRejectReason; 3] = [
    ProjectionRejectReason::ObservedWithoutEvidence,
    ProjectionRejectReason::VerifiedWithoutLocalTargetMeaning,
    ProjectionRejectReason::PositiveAssertionForBlockedClaim,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ProjectionRejectMask(u32);

impl ProjectionRejectMask {
    #[must_use]
    pub const fn contains(self, reason: ProjectionRejectReason) -> bool {
        self.0 & rejection_bit(reason) != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = ProjectionRejectReason> {
        ALL_REJECTION_REASONS
            .into_iter()
            .filter(move |reason| self.contains(*reason))
    }

    const fn insert(&mut self, reason: ProjectionRejectReason) {
        self.0 |= rejection_bit(reason);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionAdmissibility {
    Admissible,
    Rejected(ProjectionRejectMask),
}

pub const CLAIM_SEMANTIC_REVISION: &str = "seiri.claim-semantics.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentClaimProjection {
    pub semantic_revision: String,
    pub claim_id: ClaimId,
    pub assertion_kind: ClaimAssertionKind,
    pub evidence_posture: ClaimEvidencePosture,
    pub evidence_ids: Vec<EvidenceId>,
    pub meanings: Vec<MeaningAtom>,
    pub boundaries: Vec<ClaimBoundaryKind>,
    pub admissible: bool,
    pub rejection_reasons: Vec<ProjectionRejectReason>,
    pub underclaim_loss_x100: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimAssertion {
    pub kind: ClaimAssertionKind,
    pub route: RouteKind,
    pub state: RouteState,
}

impl ClaimAssertion {
    #[must_use]
    pub fn render_sentence(self) -> String {
        let route = route_slug(self.route);
        match self.kind {
            ClaimAssertionKind::RepositoryLocalTargetObserved => format!(
                "The audit observed the `{route}` route and found its repository-local target present."
            ),
            ClaimAssertionKind::StructuredRouteObserved => {
                format!("The audit observed structured evidence for the `{route}` route.")
            }
            ClaimAssertionKind::ReadmeRouteObserved => {
                format!("The audit observed a README entry for the `{route}` route.")
            }
            ClaimAssertionKind::RouteGapObserved => format!(
                "Within the scanned scope, the audit did not observe the `{route}` route."
            ),
            ClaimAssertionKind::EvidenceScopedRouteState => format!(
                "The evidence records the `{route}` route as `{}`.",
                state_slug(self.state)
            ),
            ClaimAssertionKind::ReviewCandidate => {
                format!("Evidence identifies the `{route}` route as a review candidate.")
            }
            ClaimAssertionKind::HeldForHumanReview => {
                format!("Evidence leaves the `{route}` route for human review.")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectedAssertionLevel {
    Omitted,
    BoundaryOnly,
    HedgedBelowEvidence,
    EvidenceMatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimProjectionCandidate {
    pub assertion_level: ProjectedAssertionLevel,
    pub evidence_linked: bool,
    pub positive_first: bool,
}

impl ClaimProjectionCandidate {
    #[must_use]
    pub const fn evidence_matched() -> Self {
        Self {
            assertion_level: ProjectedAssertionLevel::EvidenceMatched,
            evidence_linked: true,
            positive_first: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct UnderclaimCauseMask(u8);

impl UnderclaimCauseMask {
    pub const MISSING_POSITIVE_ASSERTION: Self = Self(1 << 0);
    pub const OBSERVED_DOWNGRADED: Self = Self(1 << 1);
    pub const EVIDENCE_OMITTED: Self = Self(1 << 2);
    pub const BOUNDARY_FIRST: Self = Self(1 << 3);

    #[must_use]
    pub const fn contains(self, cause: Self) -> bool {
        self.0 & cause.0 != 0
    }

    const fn insert(&mut self, cause: Self) {
        self.0 |= cause.0;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnderclaimLoss {
    score_x100: u8,
    causes: UnderclaimCauseMask,
}

impl UnderclaimLoss {
    #[must_use]
    pub const fn score_x100(self) -> u8 {
        self.score_x100
    }

    #[must_use]
    pub const fn causes(self) -> UnderclaimCauseMask {
        self.causes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibratedClaimProjection {
    pub assertion: ClaimAssertion,
    pub evidence_posture: ClaimEvidencePosture,
    pub meanings: MeaningMask,
    pub boundaries: ClaimBoundaryMask,
    pub admissibility: ProjectionAdmissibility,
    pub underclaim_loss: UnderclaimLoss,
}

#[must_use]
pub fn calibrate_content_claim(claim: &ContentClaim) -> CalibratedClaimProjection {
    let meanings = MeaningMask::from_atoms(&claim.allowed_meanings);
    let assertion = ClaimAssertion {
        kind: assertion_kind(claim),
        route: claim.route,
        state: claim.state,
    };
    let admissibility = evaluate_projection_admissibility(claim, assertion, meanings);
    CalibratedClaimProjection {
        assertion,
        evidence_posture: evidence_posture(claim.strength),
        meanings,
        boundaries: resolve_claim_boundaries(claim.route, claim.state, claim.strength, meanings),
        admissibility,
        underclaim_loss: if matches!(admissibility, ProjectionAdmissibility::Admissible) {
            evaluate_underclaim_loss(claim, ClaimProjectionCandidate::evidence_matched())
        } else {
            UnderclaimLoss::default()
        },
    }
}

#[must_use]
pub fn project_content_claim(claim: &ContentClaim) -> ContentClaimProjection {
    let projection = calibrate_content_claim(claim);
    let (admissible, rejection_reasons) = match projection.admissibility {
        ProjectionAdmissibility::Admissible => (true, Vec::new()),
        ProjectionAdmissibility::Rejected(reasons) => (false, reasons.iter().collect()),
    };
    ContentClaimProjection {
        semantic_revision: CLAIM_SEMANTIC_REVISION.to_string(),
        claim_id: claim.id().clone(),
        assertion_kind: projection.assertion.kind,
        evidence_posture: projection.evidence_posture,
        evidence_ids: claim.evidence_ids().to_vec(),
        meanings: projection.meanings.iter().collect(),
        boundaries: projection.boundaries.to_vec(),
        admissible,
        rejection_reasons,
        underclaim_loss_x100: projection.underclaim_loss.score_x100(),
    }
}

#[must_use]
pub fn evaluate_projection_admissibility(
    claim: &ContentClaim,
    assertion: ClaimAssertion,
    meanings: MeaningMask,
) -> ProjectionAdmissibility {
    let mut reasons = ProjectionRejectMask::default();
    if claim.strength == ClaimStrength::Observed && claim.evidence_ids.is_empty() {
        reasons.insert(ProjectionRejectReason::ObservedWithoutEvidence);
    }
    if assertion.kind == ClaimAssertionKind::RepositoryLocalTargetObserved
        && !meanings.contains(MeaningAtom::RepositoryLocalTargetPresent)
    {
        reasons.insert(ProjectionRejectReason::VerifiedWithoutLocalTargetMeaning);
    }
    if claim.strength == ClaimStrength::Blocked && assertion.kind.has_primary_assertion() {
        reasons.insert(ProjectionRejectReason::PositiveAssertionForBlockedClaim);
    }
    if reasons.is_empty() {
        ProjectionAdmissibility::Admissible
    } else {
        ProjectionAdmissibility::Rejected(reasons)
    }
}

const fn rejection_bit(reason: ProjectionRejectReason) -> u32 {
    match reason {
        ProjectionRejectReason::ObservedWithoutEvidence => 1 << 0,
        ProjectionRejectReason::VerifiedWithoutLocalTargetMeaning => 1 << 1,
        ProjectionRejectReason::PositiveAssertionForBlockedClaim => 1 << 2,
    }
}

#[must_use]
pub fn evaluate_underclaim_loss(
    claim: &ContentClaim,
    candidate: ClaimProjectionCandidate,
) -> UnderclaimLoss {
    let mut score = 0u8;
    let mut causes = UnderclaimCauseMask::default();

    if matches!(
        candidate.assertion_level,
        ProjectedAssertionLevel::Omitted | ProjectedAssertionLevel::BoundaryOnly
    ) {
        score = score.saturating_add(55);
        causes.insert(UnderclaimCauseMask::MISSING_POSITIVE_ASSERTION);
    }
    if claim.strength == ClaimStrength::Observed
        && matches!(
            candidate.assertion_level,
            ProjectedAssertionLevel::HedgedBelowEvidence
                | ProjectedAssertionLevel::BoundaryOnly
                | ProjectedAssertionLevel::Omitted
        )
    {
        score = score.saturating_add(35);
        causes.insert(UnderclaimCauseMask::OBSERVED_DOWNGRADED);
    }
    if !candidate.evidence_linked && !claim.evidence_ids.is_empty() {
        score = score.saturating_add(25);
        causes.insert(UnderclaimCauseMask::EVIDENCE_OMITTED);
    }
    if !candidate.positive_first
        && !matches!(candidate.assertion_level, ProjectedAssertionLevel::Omitted)
    {
        score = score.saturating_add(20);
        causes.insert(UnderclaimCauseMask::BOUNDARY_FIRST);
    }

    UnderclaimLoss {
        score_x100: score.min(100),
        causes,
    }
}

fn assertion_kind(claim: &ContentClaim) -> ClaimAssertionKind {
    if claim.strength == ClaimStrength::Suggested {
        return ClaimAssertionKind::ReviewCandidate;
    }
    if claim.strength == ClaimStrength::Blocked || claim.state == RouteState::UnsafeToInvent {
        return ClaimAssertionKind::HeldForHumanReview;
    }
    match claim.state {
        RouteState::Verified => ClaimAssertionKind::RepositoryLocalTargetObserved,
        RouteState::Structured => ClaimAssertionKind::StructuredRouteObserved,
        RouteState::Routed => ClaimAssertionKind::ReadmeRouteObserved,
        RouteState::Absent => ClaimAssertionKind::RouteGapObserved,
        RouteState::Implicit
        | RouteState::Weak
        | RouteState::Inherited
        | RouteState::Overridden
        | RouteState::Conflicting
        | RouteState::Overloaded
        | RouteState::Stale => ClaimAssertionKind::EvidenceScopedRouteState,
        RouteState::UnsafeToInvent => ClaimAssertionKind::HeldForHumanReview,
    }
}

const fn evidence_posture(strength: ClaimStrength) -> ClaimEvidencePosture {
    match strength {
        ClaimStrength::Observed => ClaimEvidencePosture::DirectObservation,
        ClaimStrength::Inferred => ClaimEvidencePosture::BoundedInference,
        ClaimStrength::Suggested => ClaimEvidencePosture::ReviewSuggestion,
        ClaimStrength::Blocked => ClaimEvidencePosture::BlockedForHumanReview,
    }
}

const fn route_slug(route: RouteKind) -> &'static str {
    route.slug()
}

const fn state_slug(state: RouteState) -> &'static str {
    match state {
        RouteState::Absent => "absent",
        RouteState::Implicit => "implicit",
        RouteState::Weak => "weak",
        RouteState::Routed => "routed",
        RouteState::Structured => "structured",
        RouteState::Verified => "verified",
        RouteState::Inherited => "inherited",
        RouteState::Overridden => "overridden",
        RouteState::Conflicting => "conflicting",
        RouteState::Overloaded => "overloaded",
        RouteState::Stale => "stale",
        RouteState::UnsafeToInvent => "unsafe_to_invent",
    }
}
