use super::{resolve_claim_boundaries, ClaimBoundaryMask, MeaningMask};
use crate::{ClaimStrength, ContentClaim, RouteKind, RouteState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimEvidencePosture {
    DirectObservation,
    BoundedInference,
    ReviewSuggestion,
    BlockedForHumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const fn is_positive(self) -> bool {
        !matches!(self, Self::HeldForHumanReview)
    }
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
    CalibratedClaimProjection {
        assertion,
        evidence_posture: evidence_posture(claim.strength),
        meanings,
        boundaries: resolve_claim_boundaries(claim.route, claim.state, claim.strength, meanings),
        underclaim_loss: evaluate_underclaim_loss(
            claim,
            ClaimProjectionCandidate::evidence_matched(),
        ),
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
    match route {
        RouteKind::Identity => "identity",
        RouteKind::Docs => "docs",
        RouteKind::Quickstart => "quickstart",
        RouteKind::Support => "support",
        RouteKind::Intake => "intake",
        RouteKind::Contributing => "contributing",
        RouteKind::Security => "security",
        RouteKind::Release => "release",
        RouteKind::Lifecycle => "lifecycle",
        RouteKind::Governance => "governance",
        RouteKind::License => "license",
        RouteKind::Automation => "automation",
        RouteKind::Ownership => "ownership",
        RouteKind::Hygiene => "hygiene",
        RouteKind::Unknown => "unknown",
    }
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
