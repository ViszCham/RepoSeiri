use crate::{
    EvidenceConfidence, EvidenceId, ReadmeRouteTarget, ReadmeRouteTargetStatus, RouteKind,
    RouteState,
};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePresenceAssessment {
    pub(crate) root_structured: bool,
    pub(crate) inherited: bool,
}

impl RoutePresenceAssessment {
    #[must_use]
    pub const fn root_structured(self) -> bool {
        self.root_structured
    }

    #[must_use]
    pub const fn inherited(self) -> bool {
        self.inherited
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeRoutingAssessment {
    pub(crate) candidate_count: usize,
    pub(crate) heading_count: usize,
    pub(crate) link_count: usize,
    pub(crate) badge_count: usize,
    pub(crate) target_count: usize,
}

impl ReadmeRoutingAssessment {
    pub fn new(
        candidate_count: usize,
        heading_count: usize,
        link_count: usize,
        badge_count: usize,
        target_count: usize,
    ) -> Result<Self, RouteAssessmentError> {
        let classified_count = heading_count
            .checked_add(link_count)
            .and_then(|count| count.checked_add(badge_count))
            .ok_or(RouteAssessmentError::CountOverflow)?;
        if candidate_count != classified_count {
            return Err(RouteAssessmentError::CandidateCountMismatch {
                candidate_count,
                classified_count,
            });
        }
        Ok(Self {
            candidate_count,
            heading_count,
            link_count,
            badge_count,
            target_count,
        })
    }

    #[must_use]
    pub const fn is_present(self) -> bool {
        self.candidate_count > 0
    }

    #[must_use]
    pub const fn is_overloaded(self) -> bool {
        self.candidate_count >= 4 || self.target_count >= 4
    }

    #[must_use]
    pub const fn candidate_count(self) -> usize {
        self.candidate_count
    }

    #[must_use]
    pub const fn heading_count(self) -> usize {
        self.heading_count
    }

    #[must_use]
    pub const fn link_count(self) -> usize {
        self.link_count
    }

    #[must_use]
    pub const fn badge_count(self) -> usize {
        self.badge_count
    }

    #[must_use]
    pub const fn target_count(self) -> usize {
        self.target_count
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetReachabilityAssessment {
    pub(crate) repository_local_present: usize,
    pub(crate) repository_local_missing: usize,
    pub(crate) external: usize,
    pub(crate) anchor: usize,
    pub(crate) mail: usize,
    pub(crate) unknown: usize,
}

impl TargetReachabilityAssessment {
    #[must_use]
    pub fn from_targets(targets: &[ReadmeRouteTarget]) -> Self {
        let mut assessment = Self::default();
        for target in targets {
            match target.status {
                ReadmeRouteTargetStatus::LocalPresent => {
                    assessment.repository_local_present += 1;
                }
                ReadmeRouteTargetStatus::LocalMissing => {
                    assessment.repository_local_missing += 1;
                }
                ReadmeRouteTargetStatus::External => assessment.external += 1,
                ReadmeRouteTargetStatus::Anchor => assessment.anchor += 1,
                ReadmeRouteTargetStatus::Mail => assessment.mail += 1,
                ReadmeRouteTargetStatus::Unknown => assessment.unknown += 1,
            }
        }
        assessment
    }

    #[must_use]
    pub const fn freshness(self) -> RouteFreshness {
        match (
            self.repository_local_present > 0,
            self.repository_local_missing > 0,
        ) {
            (false, false) => RouteFreshness::NotApplicable,
            (true, false) => RouteFreshness::Current,
            (false, true) => RouteFreshness::Stale,
            (true, true) => RouteFreshness::Mixed,
        }
    }

    #[must_use]
    pub const fn repository_local_present(self) -> usize {
        self.repository_local_present
    }

    #[must_use]
    pub const fn repository_local_missing(self) -> usize {
        self.repository_local_missing
    }

    #[must_use]
    pub const fn external(self) -> usize {
        self.external
    }

    #[must_use]
    pub const fn anchor(self) -> usize {
        self.anchor
    }

    #[must_use]
    pub const fn mail(self) -> usize {
        self.mail
    }

    #[must_use]
    pub const fn unknown(self) -> usize {
        self.unknown
    }

    #[must_use]
    pub const fn non_local_or_unknown(self) -> usize {
        self.external + self.anchor + self.mail + self.unknown
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteConflictAssessment {
    pub(crate) shared_target_count: usize,
}

impl RouteConflictAssessment {
    #[must_use]
    pub const fn shared_target_count(self) -> usize {
        self.shared_target_count
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteFreshness {
    #[default]
    NotApplicable,
    Current,
    Stale,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReadmeRouteAssessment {
    pub(crate) routing: ReadmeRoutingAssessment,
    pub(crate) target_reachability: TargetReachabilityAssessment,
    pub(crate) conflict: RouteConflictAssessment,
    pub(crate) freshness: RouteFreshness,
}

impl ReadmeRouteAssessment {
    pub fn from_observations(
        candidate_count: usize,
        heading_count: usize,
        link_count: usize,
        badge_count: usize,
        target_count: usize,
        targets: &[ReadmeRouteTarget],
    ) -> Result<Self, RouteAssessmentError> {
        let routing = ReadmeRoutingAssessment::new(
            candidate_count,
            heading_count,
            link_count,
            badge_count,
            target_count,
        )?;
        let target_reachability = TargetReachabilityAssessment::from_targets(targets);
        let conflict = RouteConflictAssessment {
            shared_target_count: targets
                .iter()
                .filter(|target| target.routes.len() > 1)
                .count(),
        };
        Ok(Self {
            routing,
            target_reachability,
            conflict,
            freshness: target_reachability.freshness(),
        })
    }

    #[must_use]
    pub fn summary_state(self, route: RouteKind) -> RouteState {
        if !self.routing.is_present() {
            RouteState::Absent
        } else if matches!(
            self.freshness,
            RouteFreshness::Stale | RouteFreshness::Mixed
        ) {
            RouteState::Stale
        } else if self.conflict.shared_target_count > 0 {
            RouteState::Conflicting
        } else if self.routing.is_overloaded() {
            RouteState::Overloaded
        } else if self.target_reachability.repository_local_present > 0 {
            RouteState::Verified
        } else if self.routing.target_count > 0 || route == RouteKind::Quickstart {
            RouteState::Routed
        } else {
            RouteState::Weak
        }
    }

    #[must_use]
    pub fn summary_reason(self, route: RouteKind) -> &'static str {
        match self.summary_state(route) {
            RouteState::Absent => "No README evidence was observed for this route.",
            RouteState::Weak => "README route evidence is visible but does not expose a target.",
            RouteState::Conflicting => {
                "README links reuse a target across multiple route kinds, so route intent is ambiguous."
            }
            RouteState::Overloaded => {
                "README exposes many entries for this route; users may need a clearer single path."
            }
            RouteState::Stale => {
                "README links to a local target that was not found in the repository."
            }
            RouteState::Verified if route == RouteKind::Quickstart => {
                "README exposes a reachable first-run path."
            }
            RouteState::Verified if route == RouteKind::Lifecycle => {
                "README exposes a reachable lifecycle, maintenance, deprecation, or supported-version route."
            }
            RouteState::Verified => {
                "README exposes an existence-checked repository-local route target."
            }
            RouteState::Routed => {
                "README exposes this route without an existence-checked repository-local target."
            }
            _ => "README route map emitted this state from observed route evidence.",
        }
    }

    #[must_use]
    pub const fn routing(self) -> ReadmeRoutingAssessment {
        self.routing
    }

    #[must_use]
    pub const fn target_reachability(self) -> TargetReachabilityAssessment {
        self.target_reachability
    }

    #[must_use]
    pub const fn conflict(self) -> RouteConflictAssessment {
        self.conflict
    }

    #[must_use]
    pub const fn freshness(self) -> RouteFreshness {
        self.freshness
    }
}

impl Default for ReadmeRouteAssessment {
    fn default() -> Self {
        Self {
            routing: ReadmeRoutingAssessment::default(),
            target_reachability: TargetReachabilityAssessment::default(),
            conflict: RouteConflictAssessment::default(),
            freshness: RouteFreshness::NotApplicable,
        }
    }
}

impl<'de> Deserialize<'de> for ReadmeRouteAssessment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireAssessment {
            routing: ReadmeRoutingAssessment,
            target_reachability: TargetReachabilityAssessment,
            conflict: RouteConflictAssessment,
            freshness: RouteFreshness,
        }

        let wire = WireAssessment::deserialize(deserializer)?;
        ReadmeRoutingAssessment::new(
            wire.routing.candidate_count,
            wire.routing.heading_count,
            wire.routing.link_count,
            wire.routing.badge_count,
            wire.routing.target_count,
        )
        .map_err(D::Error::custom)?;
        let expected_freshness = wire.target_reachability.freshness();
        if wire.freshness != expected_freshness {
            return Err(D::Error::custom(RouteAssessmentError::FreshnessMismatch {
                expected: expected_freshness,
                actual: wire.freshness,
            }));
        }
        Ok(Self {
            routing: wire.routing,
            target_reachability: wire.target_reachability,
            conflict: wire.conflict,
            freshness: wire.freshness,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePolicyBoundary {
    Suggestible,
    MaintainerDecisionRequired,
}

impl RoutePolicyBoundary {
    #[must_use]
    pub const fn for_route(route: RouteKind) -> Self {
        if matches!(
            route,
            RouteKind::License
                | RouteKind::Security
                | RouteKind::Lifecycle
                | RouteKind::Governance
                | RouteKind::Ownership
        ) {
            Self::MaintainerDecisionRequired
        } else {
            Self::Suggestible
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEvidenceGroups {
    pub(crate) root_structural: Vec<EvidenceId>,
    pub(crate) readme_routing: Vec<EvidenceId>,
    pub(crate) inherited: Vec<EvidenceId>,
}

impl RouteEvidenceGroups {
    pub fn new(
        mut root_structural: Vec<EvidenceId>,
        mut readme_routing: Vec<EvidenceId>,
        mut inherited: Vec<EvidenceId>,
    ) -> Result<Self, RouteAssessmentError> {
        normalize_ids(&mut root_structural);
        normalize_ids(&mut readme_routing);
        normalize_ids(&mut inherited);
        let groups = Self {
            root_structural,
            readme_routing,
            inherited,
        };
        groups.validate_disjoint()?;
        Ok(groups)
    }

    #[must_use]
    pub fn presence(&self) -> RoutePresenceAssessment {
        RoutePresenceAssessment {
            root_structured: !self.root_structural.is_empty(),
            inherited: !self.inherited.is_empty(),
        }
    }

    fn validate_disjoint(&self) -> Result<(), RouteAssessmentError> {
        let mut observed = BTreeSet::new();
        for id in self
            .root_structural
            .iter()
            .chain(&self.readme_routing)
            .chain(&self.inherited)
        {
            if !observed.insert(*id) {
                return Err(RouteAssessmentError::EvidenceGroupOverlap { id: *id });
            }
        }
        Ok(())
    }

    fn is_canonical(&self) -> bool {
        is_sorted_unique(&self.root_structural)
            && is_sorted_unique(&self.readme_routing)
            && is_sorted_unique(&self.inherited)
    }

    #[must_use]
    pub fn root_structural(&self) -> &[EvidenceId] {
        &self.root_structural
    }

    #[must_use]
    pub fn readme_routing(&self) -> &[EvidenceId] {
        &self.readme_routing
    }

    #[must_use]
    pub fn inherited(&self) -> &[EvidenceId] {
        &self.inherited
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteAssessment {
    pub(crate) route: RouteKind,
    pub(crate) presence: RoutePresenceAssessment,
    pub(crate) readme: ReadmeRouteAssessment,
    pub(crate) policy: RoutePolicyBoundary,
    pub(crate) missing_pattern: bool,
    pub(crate) evidence: RouteEvidenceGroups,
}

impl RouteAssessment {
    pub fn new(
        route: RouteKind,
        readme: ReadmeRouteAssessment,
        missing_pattern: bool,
        root_structural_evidence: Vec<EvidenceId>,
        readme_routing_evidence: Vec<EvidenceId>,
        inherited_evidence: Vec<EvidenceId>,
    ) -> Result<Self, RouteAssessmentError> {
        let evidence = RouteEvidenceGroups::new(
            root_structural_evidence,
            readme_routing_evidence,
            inherited_evidence,
        )?;
        Ok(Self {
            route,
            presence: evidence.presence(),
            readme,
            policy: RoutePolicyBoundary::for_route(route),
            missing_pattern,
            evidence,
        })
    }

    #[must_use]
    pub fn summary_projection(&self) -> RouteSummaryProjection {
        let readme_state = self.readme.summary_state(self.route);
        let identity_verified = self.route == RouteKind::Identity
            && self.presence.root_structured
            && self.readme.routing.is_present();

        if matches!(readme_state, RouteState::Stale) {
            return RouteSummaryProjection::new(
                RouteState::Stale,
                EvidenceConfidence::Medium,
                self.readme.summary_reason(self.route),
            );
        }
        if matches!(readme_state, RouteState::Conflicting) {
            return RouteSummaryProjection::new(
                RouteState::Conflicting,
                EvidenceConfidence::Medium,
                self.readme.summary_reason(self.route),
            );
        }
        if matches!(readme_state, RouteState::Overloaded) {
            return RouteSummaryProjection::new(
                RouteState::Overloaded,
                EvidenceConfidence::Medium,
                self.readme.summary_reason(self.route),
            );
        }
        if !self.presence.root_structured && readme_state == RouteState::Weak {
            return RouteSummaryProjection::new(
                RouteState::Weak,
                EvidenceConfidence::Low,
                self.readme.summary_reason(self.route),
            );
        }
        if self.presence.root_structured
            && (self.readme.target_reachability.repository_local_present > 0 || identity_verified)
        {
            return RouteSummaryProjection::new(
                RouteState::Verified,
                EvidenceConfidence::High,
                if identity_verified {
                    "Root README identity evidence is repository-local and structurally present."
                } else {
                    "Root structured evidence and an existence-checked repository-local README target agree."
                },
            );
        }
        if self.presence.root_structured && self.readme.routing.is_present() {
            return RouteSummaryProjection::new(
                RouteState::Structured,
                EvidenceConfidence::High,
                "Root structured evidence is present, but the README route has no existence-checked repository-local target.",
            );
        }
        if self.presence.root_structured {
            return RouteSummaryProjection::new(
                RouteState::Structured,
                EvidenceConfidence::High,
                "Root structured evidence is present, but README routing is not explicit.",
            );
        }
        if self.readme.routing.is_present() {
            return RouteSummaryProjection::new(
                RouteState::Routed,
                EvidenceConfidence::Medium,
                "README routing evidence is present.",
            );
        }
        if self.presence.inherited {
            return RouteSummaryProjection::new(
                RouteState::Inherited,
                EvidenceConfidence::Low,
                "Only non-root or fixture evidence was observed; it is not credited as a root route.",
            );
        }
        if self.missing_pattern && self.policy == RoutePolicyBoundary::MaintainerDecisionRequired {
            return RouteSummaryProjection::new(
                RouteState::UnsafeToInvent,
                EvidenceConfidence::Medium,
                "The route is missing and requires a maintainer policy or content decision.",
            );
        }
        RouteSummaryProjection::new(
            RouteState::Absent,
            EvidenceConfidence::Low,
            "No root route evidence was observed.",
        )
    }

    #[must_use]
    pub fn summary_evidence_ids(&self) -> Vec<EvidenceId> {
        let projection = self.summary_projection();
        let mut ids = if projection.state == RouteState::Inherited {
            self.evidence.inherited.clone()
        } else {
            self.evidence.root_structural.clone()
        };
        if projection.state != RouteState::Inherited {
            ids.extend(self.evidence.readme_routing.iter().copied());
        }
        normalize_ids(&mut ids);
        ids
    }

    #[must_use]
    pub const fn route(&self) -> RouteKind {
        self.route
    }

    #[must_use]
    pub const fn presence(&self) -> RoutePresenceAssessment {
        self.presence
    }

    #[must_use]
    pub const fn readme(&self) -> ReadmeRouteAssessment {
        self.readme
    }

    #[must_use]
    pub const fn policy(&self) -> RoutePolicyBoundary {
        self.policy
    }

    #[must_use]
    pub const fn missing_pattern(&self) -> bool {
        self.missing_pattern
    }

    #[must_use]
    pub const fn evidence(&self) -> &RouteEvidenceGroups {
        &self.evidence
    }
}

impl<'de> Deserialize<'de> for RouteAssessment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireAssessment {
            route: RouteKind,
            presence: RoutePresenceAssessment,
            readme: ReadmeRouteAssessment,
            policy: RoutePolicyBoundary,
            missing_pattern: bool,
            evidence: RouteEvidenceGroups,
        }

        let wire = WireAssessment::deserialize(deserializer)?;
        if !wire.evidence.is_canonical() {
            return Err(D::Error::custom(
                RouteAssessmentError::NonCanonicalEvidenceOrder,
            ));
        }
        wire.evidence
            .validate_disjoint()
            .map_err(D::Error::custom)?;
        let expected_presence = wire.evidence.presence();
        if wire.presence != expected_presence {
            return Err(D::Error::custom(RouteAssessmentError::PresenceMismatch));
        }
        let expected_policy = RoutePolicyBoundary::for_route(wire.route);
        if wire.policy != expected_policy {
            return Err(D::Error::custom(RouteAssessmentError::PolicyMismatch));
        }
        Ok(Self {
            route: wire.route,
            presence: wire.presence,
            readme: wire.readme,
            policy: wire.policy,
            missing_pattern: wire.missing_pattern,
            evidence: wire.evidence,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSummaryProjection {
    pub state: RouteState,
    pub confidence: EvidenceConfidence,
    pub reason: &'static str,
}

impl RouteSummaryProjection {
    const fn new(state: RouteState, confidence: EvidenceConfidence, reason: &'static str) -> Self {
        Self {
            state,
            confidence,
            reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAssessmentError {
    CountOverflow,
    CandidateCountMismatch {
        candidate_count: usize,
        classified_count: usize,
    },
    FreshnessMismatch {
        expected: RouteFreshness,
        actual: RouteFreshness,
    },
    EvidenceGroupOverlap {
        id: EvidenceId,
    },
    NonCanonicalEvidenceOrder,
    PresenceMismatch,
    PolicyMismatch,
}

impl Display for RouteAssessmentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CountOverflow => formatter.write_str("route assessment count overflow"),
            Self::CandidateCountMismatch {
                candidate_count,
                classified_count,
            } => write!(
                formatter,
                "README candidate count {candidate_count} does not match classified source count {classified_count}"
            ),
            Self::FreshnessMismatch { expected, actual } => write!(
                formatter,
                "route freshness {actual:?} does not match target reachability {expected:?}"
            ),
            Self::EvidenceGroupOverlap { id } => {
                write!(formatter, "evidence id {id} appears in multiple route groups")
            }
            Self::NonCanonicalEvidenceOrder => formatter.write_str(
                "route assessment evidence ids must be sorted and unique within each group",
            ),
            Self::PresenceMismatch => formatter.write_str(
                "route presence does not match root and inherited evidence groups",
            ),
            Self::PolicyMismatch => {
                formatter.write_str("route policy boundary does not match the route kind")
            }
        }
    }
}

impl std::error::Error for RouteAssessmentError {}

fn normalize_ids(ids: &mut Vec<EvidenceId>) {
    ids.sort();
    ids.dedup();
}

fn is_sorted_unique(ids: &[EvidenceId]) -> bool {
    ids.windows(2).all(|pair| pair[0] < pair[1])
}
