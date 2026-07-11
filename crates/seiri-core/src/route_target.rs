use crate::{DocumentId, EvidenceId, RouteKind, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum RouteTargetRole {
    Canonical,
    Detail,
    Example,
    Alternate,
    Migration,
    SharedHub,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetRelation {
    Equivalent,
    Refines,
    SharedHub,
    Competes,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteTargetRef {
    pub route: RouteKind,
    pub document: DocumentId,
    pub evidence: EvidenceId,
    pub span: SourceSpan,
    pub role: RouteTargetRole,
    pub normalized_target: String,
}

#[must_use]
pub fn classify_target_relation(left: &RouteTargetRef, right: &RouteTargetRef) -> TargetRelation {
    if left.route != right.route {
        return TargetRelation::Unknown;
    }
    if left.normalized_target == right.normalized_target {
        return TargetRelation::Equivalent;
    }
    if matches!(left.role, RouteTargetRole::SharedHub)
        || matches!(right.role, RouteTargetRole::SharedHub)
    {
        return TargetRelation::SharedHub;
    }
    if matches!(left.role, RouteTargetRole::Canonical)
        && matches!(right.role, RouteTargetRole::Canonical)
    {
        return TargetRelation::Competes;
    }
    if is_refinement_role(left.role) || is_refinement_role(right.role) {
        return TargetRelation::Refines;
    }
    TargetRelation::Unknown
}

const fn is_refinement_role(role: RouteTargetRole) -> bool {
    matches!(
        role,
        RouteTargetRole::Detail | RouteTargetRole::Example | RouteTargetRole::Migration
    )
}
