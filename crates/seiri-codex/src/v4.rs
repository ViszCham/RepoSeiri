pub(crate) mod actions;
mod linter;
mod render;
pub(crate) mod summary;

use seiri_core::{
    CodexLinterContext, CodexNativeAction, CodexNativeReviewContext, CodexQueryData,
    CodexQueryKind, CodexQueryView, CodexRoutesQuery, CodexSummaryQuery, PatchPlan, RepoSnapshot,
    WordingLintReport, CODEX_KERNEL_SCHEMA_VERSION, CODEX_NATIVE_SCHEMA_VERSION,
    CODEX_QUERY_SCHEMA_VERSION, TOOL_NAME,
};

use actions::build_native_actions;
use linter::build_linter_context;
use summary::{native_audit_summary, native_route_summary};

pub use render::{
    render_linter_context_markdown, render_native_context_markdown, render_query_view_markdown,
};

#[derive(Debug, Clone)]
pub struct CodexReviewKernel {
    pub(super) snapshot: RepoSnapshot,
    pub(super) plan: PatchPlan,
    pub(super) wording_lint: Option<WordingLintReport>,
    pub(super) linter: CodexLinterContext,
    pub(super) actions: Vec<CodexNativeAction>,
}

impl CodexReviewKernel {
    #[must_use]
    pub fn new(
        snapshot: &RepoSnapshot,
        plan: &PatchPlan,
        wording_lint: Option<&WordingLintReport>,
    ) -> Self {
        let profile = snapshot.profile.as_ref().map(|profile| profile.profile);
        Self {
            snapshot: snapshot.clone(),
            plan: plan.clone(),
            wording_lint: wording_lint.cloned(),
            linter: build_linter_context(wording_lint),
            actions: build_native_actions(snapshot, profile),
        }
    }

    #[must_use]
    pub fn compatibility_v1(&self) -> seiri_core::CodexReviewContext {
        super::build_compatibility_view(self)
    }

    #[must_use]
    pub fn native_v2(&self) -> CodexNativeReviewContext {
        CodexNativeReviewContext {
            schema_version: CODEX_NATIVE_SCHEMA_VERSION.to_string(),
            kernel_schema_version: CODEX_KERNEL_SCHEMA_VERSION.to_string(),
            tool: TOOL_NAME.to_string(),
            repo_root: self.snapshot.repo_root.clone(),
            profile: self
                .snapshot
                .profile
                .as_ref()
                .map(|profile| profile.profile),
            audit: native_audit_summary(&self.snapshot),
            route_summary: native_route_summary(&self.snapshot),
            document: self.snapshot.readme_document.clone(),
            evidence_kernel: self.snapshot.evidence_kernel.clone(),
            route_assessments: self.snapshot.route_assessments.clone(),
            claims: self.snapshot.claims.clone(),
            missing_route_priority: self.snapshot.missing_route_priority.clone(),
            plan: self.plan.clone(),
            findings: self.snapshot.findings.clone(),
            linter: self.linter.clone(),
            actions: self.actions.clone(),
            query_kinds: CodexQueryKind::ALL.to_vec(),
            calibration_sources: seiri_core::CalibrationSourceVisibilitySummary::default(),
            claim_boundary: native_claim_boundary(),
        }
    }

    #[must_use]
    pub fn query(&self, kind: CodexQueryKind) -> CodexQueryView {
        let query = match kind {
            CodexQueryKind::Summary => CodexQueryData::Summary(CodexSummaryQuery {
                audit: native_audit_summary(&self.snapshot),
                route_summary: native_route_summary(&self.snapshot),
                canonical_claims: self.snapshot.claims.len(),
                canonical_route_assessments: self.snapshot.route_assessments.len(),
                patch_operations: self.plan.operations.len(),
                blocked_patch_items: self.plan.blocked.len(),
                linter_findings: self.linter.findings.len(),
            }),
            CodexQueryKind::Routes => CodexQueryData::Routes(CodexRoutesQuery {
                assessments: self.snapshot.route_assessments.clone(),
                missing_route_priority: self.snapshot.missing_route_priority.clone(),
            }),
            CodexQueryKind::Patches => CodexQueryData::Patches(self.plan.clone()),
            CodexQueryKind::Linter => CodexQueryData::Linter(self.linter.clone()),
            CodexQueryKind::Actions => CodexQueryData::Actions(self.actions.clone()),
        };
        CodexQueryView {
            schema_version: CODEX_QUERY_SCHEMA_VERSION.to_string(),
            kernel_schema_version: CODEX_KERNEL_SCHEMA_VERSION.to_string(),
            repo_root: self.snapshot.repo_root.clone(),
            profile: self.snapshot.profile.as_ref().map(|profile| profile.profile),
            query,
            claim_boundary: "Codex query views are bounded projections from the same typed review kernel. They do not mutate files, execute commands, adopt policy, or guarantee repository outcomes."
                .to_string(),
        }
    }

    #[must_use]
    pub fn linter_context(&self) -> CodexLinterContext {
        self.linter.clone()
    }
}

fn native_claim_boundary() -> String {
    "Codex native v2 is a typed review view over canonical document, evidence, route-assessment, claim, patch-plan, and linter data. It does not execute argv, write files, call GitHub, adopt policy, or guarantee popularity, trust, security, quality, or publication readiness."
        .to_string()
}
