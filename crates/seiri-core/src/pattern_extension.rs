use crate::{ClaimBoundaryKind, EvidenceId, PatternGroup, RouteKind, UnknownReason};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternExtensionStatus {
    #[default]
    NotRequested,
    Applied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "reason", rename_all = "snake_case")]
pub enum PatternExtensionState {
    Present,
    Absent,
    Unknown(UnknownReason),
    Conflict,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternPackProvenance {
    pub id: String,
    pub version: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternExtensionEvaluation {
    pub pattern_id: String,
    pub group: PatternGroup,
    pub route: RouteKind,
    pub state: PatternExtensionState,
    pub evidence_ids: Vec<EvidenceId>,
    pub boundaries: Vec<ClaimBoundaryKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternExtensionReport {
    pub status: PatternExtensionStatus,
    pub pack: Option<PatternPackProvenance>,
    pub evaluations: Vec<PatternExtensionEvaluation>,
    pub boundary: String,
}

impl Default for PatternExtensionReport {
    fn default() -> Self {
        Self {
            status: PatternExtensionStatus::NotRequested,
            pack: None,
            evaluations: Vec::new(),
            boundary: "Executable pattern packs are data-only, explicitly selected candidate overlays. Unknown, conflict, disabled, and absent states remain distinct; applying a pack does not adopt policy or prove repository quality.".to_string(),
        }
    }
}
