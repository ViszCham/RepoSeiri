use seiri_core::{BaselineRequirement, GateKind, Severity};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PatternBoundary {
    pub requirement: BaselineRequirement,
    pub missing_severity: Severity,
    pub missing_gate: GateKind,
    pub missing_title: &'static str,
    pub missing_message: &'static str,
    pub recommendation_title: &'static str,
    pub recommendation_message: &'static str,
}
