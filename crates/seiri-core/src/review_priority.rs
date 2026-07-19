use crate::{
    AggregateRepositoryEstimate, CoverageScope, EvidenceId, GateKind, ProfilePriority, RouteKind,
    RouteState, Severity, UnknownReason,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewGapKind {
    Route,
    Content,
    Consistency,
    ObservationUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ReviewGap {
    Route {
        route: RouteKind,
        state: RouteState,
        baseline_pattern_ids: Vec<String>,
    },
    Content {
        route: RouteKind,
        candidate_pattern_ids: Vec<String>,
    },
    ContentSlot {
        route: RouteKind,
        slot_ids: Vec<crate::ContentSlotId>,
    },
    Consistency {
        route: Option<RouteKind>,
        gap_ids: Vec<String>,
    },
    ObservationUnknown {
        route: Option<RouteKind>,
        scope: CoverageScope,
        reason: UnknownReason,
    },
}

impl ReviewGap {
    #[must_use]
    pub const fn kind(&self) -> ReviewGapKind {
        match self {
            Self::Route { .. } => ReviewGapKind::Route,
            Self::Content { .. } | Self::ContentSlot { .. } => ReviewGapKind::Content,
            Self::Consistency { .. } => ReviewGapKind::Consistency,
            Self::ObservationUnknown { .. } => ReviewGapKind::ObservationUnknown,
        }
    }

    #[must_use]
    pub const fn route(&self) -> Option<RouteKind> {
        match self {
            Self::Route { route, .. }
            | Self::Content { route, .. }
            | Self::ContentSlot { route, .. } => Some(*route),
            Self::Consistency { route, .. } | Self::ObservationUnknown { route, .. } => *route,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPriority {
    pub rank: usize,
    pub gap: ReviewGap,
    pub gate: GateKind,
    pub severity: Severity,
    pub priority: ProfilePriority,
    pub priority_score_x100: u8,
    pub calibration_estimate: Option<AggregateRepositoryEstimate>,
    pub evidence_ids: Vec<EvidenceId>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAuthority {
    SafePreview,
    GuardedPreview,
    MaintainerDecision,
}

impl ReviewPriority {
    #[must_use]
    pub const fn authority(&self) -> ReviewAuthority {
        match self.gate {
            GateKind::Safe => ReviewAuthority::SafePreview,
            GateKind::Guarded => ReviewAuthority::GuardedPreview,
            GateKind::Manual => ReviewAuthority::MaintainerDecision,
        }
    }

    #[must_use]
    pub const fn recommendation(&self) -> &'static str {
        match self.gap {
            ReviewGap::Route { .. } => {
                "Review whether an observed repository-local target should be exposed from the README."
            }
            ReviewGap::Content { .. } | ReviewGap::ContentSlot { .. } => {
                "Review the missing content separately from route presence."
            }
            ReviewGap::Consistency { .. } => {
                "Resolve conflicting repository-local document evidence before patch planning."
            }
            ReviewGap::ObservationUnknown { .. } => {
                "Expand or repair bounded coverage before interpreting absence."
            }
        }
    }

    #[must_use]
    pub const fn boundary(&self) -> &'static str {
        match self.authority() {
            ReviewAuthority::SafePreview => {
                "A dry-run preview may be generated; no write authority is granted."
            }
            ReviewAuthority::GuardedPreview => {
                "A bounded preview requires explicit review before any external action."
            }
            ReviewAuthority::MaintainerDecision => {
                "Policy or ownership content must be supplied or approved by a maintainer."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPrioritySummary {
    pub total: usize,
    pub route_gaps: usize,
    pub content_gaps: usize,
    pub consistency_gaps: usize,
    pub observation_unknowns: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPriorityReport {
    pub summary: ReviewPrioritySummary,
    pub priorities: Vec<ReviewPriority>,
}

impl ReviewPriorityReport {
    #[must_use]
    pub fn new(priorities: Vec<ReviewPriority>) -> Self {
        let mut summary = ReviewPrioritySummary {
            total: priorities.len(),
            ..ReviewPrioritySummary::default()
        };
        for priority in &priorities {
            match priority.gap.kind() {
                ReviewGapKind::Route => summary.route_gaps += 1,
                ReviewGapKind::Content => summary.content_gaps += 1,
                ReviewGapKind::Consistency => summary.consistency_gaps += 1,
                ReviewGapKind::ObservationUnknown => summary.observation_unknowns += 1,
            }
        }
        Self {
            summary,
            priorities,
        }
    }
}
