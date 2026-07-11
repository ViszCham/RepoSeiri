use crate::v4::{
    actions::build_native_actions,
    summary::{native_audit_summary, native_route_summary},
};
use seiri_core::{
    CodexNativeAction, CodexNativeAuditSummary, CodexNativeRouteSummary, PatchPlan,
    PatchPlanOperation, ProfileKind, RepoSnapshot, WordingLintReport, CODEX_KERNEL_SCHEMA_VERSION,
    CODEX_NATIVE_V3_SCHEMA_VERSION,
};
use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexNativeV3QueryKind {
    Summary,
    Routes,
    Evidence,
    Documents,
    Governance,
    Patches,
    Linter,
    Actions,
    Remote,
}

impl CodexNativeV3QueryKind {
    pub const ALL: [Self; 9] = [
        Self::Summary,
        Self::Routes,
        Self::Evidence,
        Self::Documents,
        Self::Governance,
        Self::Patches,
        Self::Linter,
        Self::Actions,
        Self::Remote,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Routes => "routes",
            Self::Evidence => "evidence",
            Self::Documents => "documents",
            Self::Governance => "governance",
            Self::Patches => "patches",
            Self::Linter => "linter",
            Self::Actions => "actions",
            Self::Remote => "remote",
        }
    }

    #[must_use]
    pub const fn compatibility_kind(self) -> Option<seiri_core::CodexQueryKind> {
        match self {
            Self::Summary => Some(seiri_core::CodexQueryKind::Summary),
            Self::Routes => Some(seiri_core::CodexQueryKind::Routes),
            Self::Patches => Some(seiri_core::CodexQueryKind::Patches),
            Self::Linter => Some(seiri_core::CodexQueryKind::Linter),
            Self::Actions => Some(seiri_core::CodexQueryKind::Actions),
            Self::Evidence | Self::Documents | Self::Governance | Self::Remote => None,
        }
    }
}

impl Display for CodexNativeV3QueryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

impl FromStr for CodexNativeV3QueryKind {
    type Err = CodexNativeV3QueryParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.slug() == value)
            .ok_or_else(|| CodexNativeV3QueryParseError {
                value: value.to_string(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexNativeV3QueryParseError {
    value: String,
}

impl Display for CodexNativeV3QueryParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown Codex query `{}`; expected one of: {}",
            self.value,
            CodexNativeV3QueryKind::ALL
                .iter()
                .map(|kind| kind.slug())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::error::Error for CodexNativeV3QueryParseError {}

/// Borrowed native v3 surface. It owns only the small argv action list.
#[derive(Debug)]
pub struct CodexNativeV3View<'a> {
    snapshot: &'a RepoSnapshot,
    plan: &'a PatchPlan,
    wording_lint: Option<&'a WordingLintReport>,
}

impl<'a> CodexNativeV3View<'a> {
    #[must_use]
    pub fn new(
        snapshot: &'a RepoSnapshot,
        plan: &'a PatchPlan,
        wording_lint: Option<&'a WordingLintReport>,
    ) -> Self {
        Self {
            snapshot,
            plan,
            wording_lint,
        }
    }

    #[must_use]
    pub fn query(&self, kind: CodexNativeV3QueryKind) -> CodexNativeV3QueryView<'_> {
        let query = match kind {
            CodexNativeV3QueryKind::Summary => CodexNativeV3Query::Summary(CodexNativeV3Summary {
                audit: native_audit_summary(self.snapshot),
                route_summary: native_route_summary(self.snapshot),
                evidence_v2_facts: self.snapshot.evidence_kernel_v2.facts().len(),
                indexed_documents: self.snapshot.document_index.entries().len(),
                route_content_assessments: self.snapshot.route_content.len(),
                facet_assessments: self.snapshot.facets.facets.len(),
                document_conflicts: self.snapshot.document_consistency.conflicts.len(),
                scope_nodes: self.snapshot.repository_scope.graph.nodes.len(),
                git_commit_headers: self.snapshot.repository_scope.git.commits.len(),
                github_critical_paths: self.snapshot.github_semantics.critical_paths.len(),
                patch_operations: self.plan.operations.len(),
                blocked_patch_items: self.plan.blocked.len(),
                bound_patch_operations: self
                    .plan
                    .operations
                    .iter()
                    .filter(|operation| operation.binding.is_some())
                    .count(),
            }),
            CodexNativeV3QueryKind::Routes => {
                CodexNativeV3Query::Routes(CodexNativeV3RoutesQuery {
                    assessments: &self.snapshot.route_assessments,
                    missing_route_priority: &self.snapshot.missing_route_priority,
                    review_priority: &self.snapshot.review_priority,
                })
            }
            CodexNativeV3QueryKind::Evidence => {
                CodexNativeV3Query::Evidence(CodexNativeV3EvidenceQuery {
                    kernel: &self.snapshot.evidence_kernel_v2,
                    coverage: &self.snapshot.coverage,
                })
            }
            CodexNativeV3QueryKind::Documents => {
                CodexNativeV3Query::Documents(CodexNativeV3DocumentsQuery {
                    index: &self.snapshot.document_index,
                    github_local: &self.snapshot.github_local_documents,
                    github_semantics: &self.snapshot.github_semantics,
                    repository_scope: &self.snapshot.repository_scope,
                    freshness: &self.snapshot.freshness,
                })
            }
            CodexNativeV3QueryKind::Governance => {
                CodexNativeV3Query::Governance(CodexNativeV3GovernanceQuery {
                    facets: &self.snapshot.facets,
                    consistency: &self.snapshot.document_consistency,
                    route_content: &self.snapshot.route_content,
                    route_content_v2: &self.snapshot.route_content_v2,
                    content_contract: seiri_core::route_content_contract_v2(),
                })
            }
            CodexNativeV3QueryKind::Patches => {
                CodexNativeV3Query::Patches(CodexNativeV3PatchQuery { plan: self.plan })
            }
            CodexNativeV3QueryKind::Linter => CodexNativeV3Query::Linter(self.wording_lint),
            CodexNativeV3QueryKind::Actions => {
                let profile = self
                    .snapshot
                    .profile
                    .as_ref()
                    .map(|profile| profile.profile);
                CodexNativeV3Query::Actions(build_native_actions(self.snapshot, profile))
            }
            CodexNativeV3QueryKind::Remote => {
                CodexNativeV3Query::Remote(&self.snapshot.remote_evidence)
            }
        };
        CodexNativeV3QueryView {
            schema_version: CODEX_NATIVE_V3_SCHEMA_VERSION,
            kernel_schema_version: CODEX_KERNEL_SCHEMA_VERSION,
            repo_root: &self.snapshot.repo_root,
            profile: self.snapshot.profile.as_ref().map(|profile| profile.profile),
            query,
            claim_boundary: "Codex native v3 is a borrowed, query-first view over canonical local analysis state. It does not clone repository collections for query construction, retain document source text, write files, execute commands, call GitHub, adopt policy, or guarantee repository outcomes.",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CodexNativeV3QueryView<'a> {
    pub schema_version: &'static str,
    pub kernel_schema_version: &'static str,
    pub repo_root: &'a str,
    pub profile: Option<ProfileKind>,
    pub query: CodexNativeV3Query<'a>,
    pub claim_boundary: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum CodexNativeV3Query<'a> {
    Summary(CodexNativeV3Summary),
    Routes(CodexNativeV3RoutesQuery<'a>),
    Evidence(CodexNativeV3EvidenceQuery<'a>),
    Documents(CodexNativeV3DocumentsQuery<'a>),
    Governance(CodexNativeV3GovernanceQuery<'a>),
    Patches(CodexNativeV3PatchQuery<'a>),
    Linter(Option<&'a WordingLintReport>),
    Actions(Vec<CodexNativeAction>),
    Remote(&'a seiri_core::RemoteEvidenceReport),
}

impl CodexNativeV3Query<'_> {
    #[must_use]
    pub const fn kind(&self) -> CodexNativeV3QueryKind {
        match self {
            Self::Summary(_) => CodexNativeV3QueryKind::Summary,
            Self::Routes(_) => CodexNativeV3QueryKind::Routes,
            Self::Evidence(_) => CodexNativeV3QueryKind::Evidence,
            Self::Documents(_) => CodexNativeV3QueryKind::Documents,
            Self::Governance(_) => CodexNativeV3QueryKind::Governance,
            Self::Patches(_) => CodexNativeV3QueryKind::Patches,
            Self::Linter(_) => CodexNativeV3QueryKind::Linter,
            Self::Actions(_) => CodexNativeV3QueryKind::Actions,
            Self::Remote(_) => CodexNativeV3QueryKind::Remote,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexNativeV3Summary {
    pub audit: CodexNativeAuditSummary,
    pub route_summary: CodexNativeRouteSummary,
    pub evidence_v2_facts: usize,
    pub indexed_documents: usize,
    pub route_content_assessments: usize,
    pub facet_assessments: usize,
    pub document_conflicts: usize,
    pub scope_nodes: usize,
    pub git_commit_headers: usize,
    pub github_critical_paths: usize,
    pub patch_operations: usize,
    pub blocked_patch_items: usize,
    pub bound_patch_operations: usize,
}

#[derive(Debug, Serialize)]
pub struct CodexNativeV3RoutesQuery<'a> {
    pub assessments: &'a [seiri_core::RouteAssessment],
    pub missing_route_priority: &'a seiri_core::MissingRoutePriorityReport,
    pub review_priority: &'a seiri_core::ReviewPriorityReport,
}

#[derive(Debug, Serialize)]
pub struct CodexNativeV3EvidenceQuery<'a> {
    pub kernel: &'a seiri_core::EvidenceKernelV2,
    pub coverage: &'a seiri_core::CoverageIndex,
}

#[derive(Debug, Serialize)]
pub struct CodexNativeV3DocumentsQuery<'a> {
    pub index: &'a seiri_core::DocumentIndex,
    pub github_local: &'a seiri_core::GithubLocalDocuments,
    pub github_semantics: &'a seiri_core::GithubSemanticsReport,
    pub repository_scope: &'a seiri_core::RepositoryScopeReport,
    pub freshness: &'a seiri_core::FreshnessReport,
}

#[derive(Debug, Serialize)]
pub struct CodexNativeV3GovernanceQuery<'a> {
    pub facets: &'a seiri_core::FacetReport,
    pub consistency: &'a seiri_core::DocumentConsistencyReport,
    pub route_content: &'a [seiri_core::RouteContentAssessment],
    pub route_content_v2: &'a seiri_core::RouteContentReportV2,
    pub content_contract: &'static [seiri_core::ContentSlotSpec],
}

#[derive(Debug)]
pub struct CodexNativeV3PatchQuery<'a> {
    pub plan: &'a PatchPlan,
}

impl Serialize for CodexNativeV3PatchQuery<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CodexNativeV3PatchQuery", 3)?;
        state.serialize_field("plan", self.plan)?;
        state.serialize_field("analysis_run", &self.plan.analysis_run)?;
        state.serialize_field(
            "operation_bindings",
            &CodexNativeV3OperationBindings(&self.plan.operations),
        )?;
        state.end()
    }
}

struct CodexNativeV3OperationBindings<'a>(&'a [PatchPlanOperation]);

impl Serialize for CodexNativeV3OperationBindings<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for operation in self.0 {
            sequence.serialize_element(&CodexNativeV3OperationBinding(operation))?;
        }
        sequence.end()
    }
}

struct CodexNativeV3OperationBinding<'a>(&'a PatchPlanOperation);

impl Serialize for CodexNativeV3OperationBinding<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CodexNativeV3OperationBinding", 3)?;
        state.serialize_field("operation_id", &self.0.id)?;
        state.serialize_field("path", &self.0.path)?;
        state.serialize_field("binding", &self.0.binding)?;
        state.end()
    }
}

#[must_use]
pub fn render_native_v3_query_markdown(view: &CodexNativeV3QueryView<'_>) -> String {
    let mut out = format!(
        "# RepoSeiri Codex Native v3 Query\n\n- Schema: `{}`\n- Kernel: `{}`\n- Repository: `{}`\n- Query: `{}`\n- Boundary: {}\n\n",
        view.schema_version,
        view.kernel_schema_version,
        view.repo_root,
        view.query.kind().slug(),
        view.claim_boundary,
    );
    match &view.query {
        CodexNativeV3Query::Summary(summary) => out.push_str(&format!(
            "- Entries: `{}`\n- Indexed documents: `{}`\n- Evidence v2 facts: `{}`\n- Route content assessments: `{}`\n- Facets: `{}`\n- Document conflicts: `{}`\n- Scope nodes: `{}` / Git commit headers `{}`\n- GitHub critical paths: `{}`\n- Patch operations: `{}` / bound `{}` / blocked `{}`\n",
            summary.audit.entries_scanned,
            summary.indexed_documents,
            summary.evidence_v2_facts,
            summary.route_content_assessments,
            summary.facet_assessments,
            summary.document_conflicts,
            summary.scope_nodes,
            summary.git_commit_headers,
            summary.github_critical_paths,
            summary.patch_operations,
            summary.bound_patch_operations,
            summary.blocked_patch_items,
        )),
        CodexNativeV3Query::Routes(routes) => out.push_str(&format!(
            "- Route assessments: `{}`\n- Review gaps: `{}`\n",
            routes.assessments.len(),
            routes.review_priority.priorities.len(),
        )),
        CodexNativeV3Query::Evidence(evidence) => out.push_str(&format!(
            "- Evidence facts: `{}`\n- Coverage records: `{}`\n",
            evidence.kernel.facts().len(),
            evidence.coverage.records().len(),
        )),
        CodexNativeV3Query::Documents(documents) => out.push_str(&format!(
            "- Indexed documents: `{}`\n- Local GitHub documents: `{}`\n- GitHub critical paths: `{}`\n- Scope nodes: `{}` / edges `{}`\n- Git refs: `{}` / commit headers `{}`\n",
            documents.index.entries().len(),
            documents.github_local.documents().len(),
            documents.github_semantics.critical_paths.len(),
            documents.repository_scope.graph.nodes.len(),
            documents.repository_scope.graph.edges.len(),
            documents.repository_scope.git.references.len(),
            documents.repository_scope.git.commits.len(),
        )),
        CodexNativeV3Query::Governance(governance) => out.push_str(&format!(
            "- Facets: `{}`\n- Conflicts: `{}`\n- Route content assessments: `{}`\n- Content slots v2: `{}` / registry `{}`\n- Structural JA/EN candidates: `{}`\n",
            governance.facets.facets.len(),
            governance.consistency.conflicts.len(),
            governance.route_content.len(),
            governance.route_content_v2.assessments.len(),
            governance.content_contract.len(),
            governance.route_content_v2.structural_pairs.len(),
        )),
        CodexNativeV3Query::Patches(patches) => out.push_str(&format!(
            "- Planner: `{}`\n- Analysis run bound: `{}`\n- Operations: `{}`\n",
            patches.plan.planner_version,
            patches.plan.analysis_run.is_some(),
            patches.plan.operations.len(),
        )),
        CodexNativeV3Query::Linter(report) => out.push_str(&format!(
            "- Linter available: `{}`\n",
            report.is_some(),
        )),
        CodexNativeV3Query::Actions(actions) => out.push_str(&format!(
            "- Argv actions: `{}`\n",
            actions.len(),
        )),
        CodexNativeV3Query::Remote(remote) => out.push_str(&format!(
            "- Remote evidence status: `{:?}`\n",
            remote.status,
        )),
    }
    out
}
