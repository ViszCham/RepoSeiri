mod command;

use crate::{
    CalibrationSourceVisibilitySummary, ClaimBoundaryKind, ContentClaim, DocumentScan,
    EvidenceKernel, Finding, MissingRoutePriorityReport, PatchPlan, ProfileKind, RouteAssessment,
    RouteKind, WordingLintFinding, WordingRuleKind,
};
use serde::{Deserialize, Serialize};

pub use command::{CodexCommand, CodexCommandError};

pub const CODEX_KERNEL_SCHEMA_VERSION: &str = "seiri.codex.kernel.v1";
pub const CODEX_NATIVE_SCHEMA_VERSION: &str = "seiri.codex.native.v2";
pub const CODEX_NATIVE_V3_SCHEMA_VERSION: &str = "seiri.codex.native.v3";
pub const CODEX_QUERY_SCHEMA_VERSION: &str = "seiri.codex.query.v2";
pub const CODEX_LINTER_CONTEXT_SCHEMA_VERSION: &str = "seiri.codex.linter_context.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexNativeReviewContext {
    pub schema_version: String,
    pub kernel_schema_version: String,
    pub tool: String,
    pub repo_root: String,
    pub profile: Option<ProfileKind>,
    pub audit: CodexNativeAuditSummary,
    pub route_summary: CodexNativeRouteSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentScan>,
    pub evidence_kernel: EvidenceKernel,
    pub route_assessments: Vec<RouteAssessment>,
    pub claims: Vec<ContentClaim>,
    pub missing_route_priority: MissingRoutePriorityReport,
    pub plan: PatchPlan,
    pub findings: Vec<Finding>,
    pub linter: CodexLinterContext,
    pub actions: Vec<CodexNativeAction>,
    pub query_kinds: Vec<CodexQueryKind>,
    #[serde(default)]
    pub calibration_sources: CalibrationSourceVisibilitySummary,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexNativeAuditSummary {
    pub entries_scanned: usize,
    pub document_events: usize,
    pub document_diagnostics: usize,
    pub evidence_facts: usize,
    pub route_assessments: usize,
    pub claims: usize,
    pub findings: usize,
    pub pattern_matches: usize,
    pub profile_score_x100: Option<u8>,
    pub profile_branches: usize,
    pub top_profile: Option<ProfileKind>,
    pub top_profile_confidence_x100: Option<u8>,
    pub missing_route_priorities: usize,
    pub co_occurrence_gaps: usize,
    pub top_missing_route: Option<RouteKind>,
    pub top_missing_route_priority_x100: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexNativeRouteSummary {
    pub assessments: usize,
    pub root_structured_routes: usize,
    pub readme_routed_routes: usize,
    pub routes_with_repository_local_target: usize,
    pub maintainer_decision_routes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexNativeAction {
    pub id: String,
    pub label: String,
    pub command: CodexCommand,
    pub mutates_files: bool,
    pub requires_confirmation: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexLinterContext {
    pub schema_version: String,
    pub kernel_schema_version: String,
    pub source_schema_version: Option<String>,
    pub available: bool,
    pub files_scanned: usize,
    pub generated_surfaces: usize,
    pub suppressed_boundary_exceptions: usize,
    pub findings: Vec<WordingLintFinding>,
    pub rules: Vec<WordingRuleKind>,
    pub boundary_kinds: Vec<ClaimBoundaryKind>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexQueryKind {
    Summary,
    Routes,
    Patches,
    Linter,
    Actions,
}

impl CodexQueryKind {
    pub const ALL: [Self; 5] = [
        Self::Summary,
        Self::Routes,
        Self::Patches,
        Self::Linter,
        Self::Actions,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexQueryView {
    pub schema_version: String,
    pub kernel_schema_version: String,
    pub repo_root: String,
    pub profile: Option<ProfileKind>,
    pub query: CodexQueryData,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum CodexQueryData {
    Summary(CodexSummaryQuery),
    Routes(CodexRoutesQuery),
    Patches(PatchPlan),
    Linter(CodexLinterContext),
    Actions(Vec<CodexNativeAction>),
}

impl CodexQueryData {
    #[must_use]
    pub const fn kind(&self) -> CodexQueryKind {
        match self {
            Self::Summary(_) => CodexQueryKind::Summary,
            Self::Routes(_) => CodexQueryKind::Routes,
            Self::Patches(_) => CodexQueryKind::Patches,
            Self::Linter(_) => CodexQueryKind::Linter,
            Self::Actions(_) => CodexQueryKind::Actions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexSummaryQuery {
    pub audit: CodexNativeAuditSummary,
    pub route_summary: CodexNativeRouteSummary,
    pub canonical_claims: usize,
    pub canonical_route_assessments: usize,
    pub patch_operations: usize,
    pub blocked_patch_items: usize,
    pub linter_findings: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexRoutesQuery {
    pub assessments: Vec<RouteAssessment>,
    pub missing_route_priority: MissingRoutePriorityReport,
}
