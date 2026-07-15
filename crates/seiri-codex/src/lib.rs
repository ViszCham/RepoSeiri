#![forbid(unsafe_code)]

use seiri_core::{
    calibrate_content_claim, project_content_claim, stable_id, ClaimStrength, CodexAction,
    CodexCommand, ContentClaim, ContentClaimProjection, CoverageIndex, DocumentConsistencyReport,
    DocumentIndex, EvidenceKernel, FacetReport, FreshnessReport, GithubLocalDocuments,
    GithubSemanticsReport, MissingRoutePriorityReport, PatchPlan, ProfileKind,
    RemoteEvidenceReport, RepositoryAnalysis, RepositoryScopeReport, RouteAssessment,
    RouteContentReport, WordingLintReport, CODEX_SCHEMA_VERSION,
};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexQueryKind {
    Summary,
    Routes,
    Evidence,
    Documents,
    Governance,
    Patches,
    Linter,
    Actions,
    Remote,
    PrBody,
}

impl CodexQueryKind {
    pub const ALL: [Self; 10] = [
        Self::Summary,
        Self::Routes,
        Self::Evidence,
        Self::Documents,
        Self::Governance,
        Self::Patches,
        Self::Linter,
        Self::Actions,
        Self::Remote,
        Self::PrBody,
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
            Self::PrBody => "pr-body",
        }
    }
}

impl Display for CodexQueryKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.slug())
    }
}

impl FromStr for CodexQueryKind {
    type Err = CodexQueryParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.slug() == value)
            .ok_or_else(|| CodexQueryParseError {
                value: value.to_string(),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexQueryParseError {
    value: String,
}

impl Display for CodexQueryParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unknown Codex query `{}`; expected one of: {}",
            self.value,
            CodexQueryKind::ALL
                .iter()
                .map(|kind| kind.slug())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::error::Error for CodexQueryParseError {}

#[derive(Debug)]
pub struct CodexView<'a> {
    analysis: &'a RepositoryAnalysis,
    plan: &'a PatchPlan,
    wording_lint: Option<&'a WordingLintReport>,
}

impl<'a> CodexView<'a> {
    #[must_use]
    pub const fn new(
        analysis: &'a RepositoryAnalysis,
        plan: &'a PatchPlan,
        wording_lint: Option<&'a WordingLintReport>,
    ) -> Self {
        Self {
            analysis,
            plan,
            wording_lint,
        }
    }

    #[must_use]
    pub fn query(&self, kind: CodexQueryKind) -> CodexQueryView<'_> {
        let query = match kind {
            CodexQueryKind::Summary => CodexQuery::Summary(summary(self.analysis, self.plan)),
            CodexQueryKind::Routes => CodexQuery::Routes(CodexRoutesQuery {
                assessments: &self.analysis.route_assessments,
                priorities: &self.analysis.missing_route_priority,
            }),
            CodexQueryKind::Evidence => CodexQuery::Evidence(CodexEvidenceQuery {
                kernel: &self.analysis.evidence_kernel,
                coverage: &self.analysis.coverage,
            }),
            CodexQueryKind::Documents => CodexQuery::Documents(CodexDocumentsQuery {
                index: &self.analysis.document_index,
                github: &self.analysis.github_local_documents,
            }),
            CodexQueryKind::Governance => CodexQuery::Governance(CodexGovernanceQuery {
                facets: &self.analysis.facets,
                route_content: &self.analysis.route_content,
                consistency: &self.analysis.document_consistency,
                github: &self.analysis.github_semantics,
                scope: &self.analysis.repository_scope,
                freshness: &self.analysis.freshness,
                claims: &self.analysis.claims,
                claim_projections: self
                    .analysis
                    .claims
                    .iter()
                    .map(project_content_claim)
                    .collect(),
            }),
            CodexQueryKind::Patches => CodexQuery::Patches(self.plan),
            CodexQueryKind::Linter => CodexQuery::Linter(CodexLinterQuery {
                report: self.wording_lint,
                boundary: linter_boundary(),
            }),
            CodexQueryKind::Actions => CodexQuery::Actions(build_actions(self.analysis)),
            CodexQueryKind::Remote => CodexQuery::Remote(&self.analysis.remote_evidence),
            CodexQueryKind::PrBody => CodexQuery::PrBody(build_pr_body(self.analysis, self.plan)),
        };
        CodexQueryView {
            schema_version: CODEX_SCHEMA_VERSION,
            repo_root: ".",
            profile: self.analysis.profile.as_ref().map(|profile| profile.profile),
            query,
            boundary: "Codex queries are bounded projections of canonical local analysis. They do not write files, execute commands, call GitHub, adopt policy, or guarantee popularity, trust, security, quality, or publication readiness.",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CodexQueryView<'a> {
    pub schema_version: &'static str,
    pub repo_root: &'a str,
    pub profile: Option<ProfileKind>,
    pub query: CodexQuery<'a>,
    pub boundary: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum CodexQuery<'a> {
    Summary(CodexSummary),
    Routes(CodexRoutesQuery<'a>),
    Evidence(CodexEvidenceQuery<'a>),
    Documents(CodexDocumentsQuery<'a>),
    Governance(CodexGovernanceQuery<'a>),
    Patches(&'a PatchPlan),
    Linter(CodexLinterQuery<'a>),
    Actions(Vec<CodexAction>),
    Remote(&'a RemoteEvidenceReport),
    PrBody(CodexPrBody),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CodexSummary {
    pub entries_scanned: usize,
    pub document_events: usize,
    pub document_diagnostics: usize,
    pub evidence_facts: usize,
    pub route_assessments: usize,
    pub route_content_slots: usize,
    pub claims: usize,
    pub findings: usize,
    pub pattern_matches: usize,
    pub profile_fit_score_x100: Option<u8>,
    pub profile_branches: usize,
    pub top_profile: Option<ProfileKind>,
    pub top_profile_rank_score_x100: Option<u8>,
    pub missing_route_priorities: usize,
    pub patch_operations: usize,
    pub patch_holds: usize,
}

#[derive(Debug, Serialize)]
pub struct CodexRoutesQuery<'a> {
    pub assessments: &'a [RouteAssessment],
    pub priorities: &'a MissingRoutePriorityReport,
}

#[derive(Debug, Serialize)]
pub struct CodexEvidenceQuery<'a> {
    pub kernel: &'a EvidenceKernel,
    pub coverage: &'a CoverageIndex,
}

#[derive(Debug, Serialize)]
pub struct CodexDocumentsQuery<'a> {
    pub index: &'a DocumentIndex,
    pub github: &'a GithubLocalDocuments,
}

#[derive(Debug, Serialize)]
pub struct CodexGovernanceQuery<'a> {
    pub facets: &'a FacetReport,
    pub route_content: &'a RouteContentReport,
    pub consistency: &'a DocumentConsistencyReport,
    pub github: &'a GithubSemanticsReport,
    pub scope: &'a RepositoryScopeReport,
    pub freshness: &'a FreshnessReport,
    pub claims: &'a [ContentClaim],
    pub claim_projections: Vec<ContentClaimProjection>,
}

#[derive(Debug, Serialize)]
pub struct CodexLinterQuery<'a> {
    pub report: Option<&'a WordingLintReport>,
    pub boundary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexPrBody {
    pub title: String,
    pub body: String,
    pub draft: bool,
}

fn summary(analysis: &RepositoryAnalysis, plan: &PatchPlan) -> CodexSummary {
    let (document_events, document_diagnostics) = analysis
        .document_index
        .scanned_documents()
        .filter_map(|entry| entry.scan.as_ref())
        .fold((0usize, 0usize), |(events, diagnostics), document| {
            (
                events.saturating_add(document.events().len()),
                diagnostics.saturating_add(document.diagnostics().len()),
            )
        });
    CodexSummary {
        entries_scanned: analysis.entry_count,
        document_events,
        document_diagnostics,
        evidence_facts: analysis.evidence_kernel.len(),
        route_assessments: analysis.route_assessments.len(),
        route_content_slots: analysis.route_content.assessments.len(),
        claims: analysis.claims.len(),
        findings: analysis.findings.len(),
        pattern_matches: analysis.pattern_matches.len(),
        profile_fit_score_x100: analysis
            .profile
            .as_ref()
            .map(|profile| profile.score.score_x100),
        profile_branches: analysis
            .profile
            .as_ref()
            .map_or(0, |profile| profile.branches.len()),
        top_profile: analysis
            .profile
            .as_ref()
            .and_then(|profile| profile.branch_summary.top_profile),
        top_profile_rank_score_x100: analysis
            .profile
            .as_ref()
            .and_then(|profile| profile.branch_summary.top_rank_score_x100),
        missing_route_priorities: analysis.missing_route_priority.priorities.len(),
        patch_operations: plan.operations.len(),
        patch_holds: plan.held.len(),
    }
}

fn build_actions(analysis: &RepositoryAnalysis) -> Vec<CodexAction> {
    let profile = analysis
        .profile
        .as_ref()
        .map_or(ProfileKind::Common, |profile| profile.profile)
        .to_string();
    [
        ("Render audit report", "audit", None),
        ("Render dry-run patch plan", "plan", None),
        ("Render Codex PR body", "codex", Some("pr-body")),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (label, subcommand, query))| {
        let mut args = vec![
            subcommand.to_string(),
            "--path".to_string(),
            ".".to_string(),
            "--profile".to_string(),
            profile.clone(),
            "--format".to_string(),
            "markdown".to_string(),
        ];
        if let Some(query) = query {
            args.extend(["--query".to_string(), query.to_string()]);
        }
        CodexAction {
            id: stable_id("codex-action", index + 1),
            label: label.to_string(),
            command: CodexCommand::new("seiri", args).expect("built-in argv is valid"),
            runtime: seiri_core::CodexRuntimeRequirement::default(),
            mutates_files: false,
            requires_confirmation: false,
            detail: "Review command only; RepoSeiri does not execute this argv.".to_string(),
        }
    })
    .collect()
}

fn build_pr_body(analysis: &RepositoryAnalysis, plan: &PatchPlan) -> CodexPrBody {
    let summary = summary(analysis, plan);
    let observed_claims = analysis
        .claims
        .iter()
        .filter(|claim| claim.strength() == ClaimStrength::Observed)
        .count();
    let examples = analysis
        .claims
        .iter()
        .filter(|claim| claim.strength() == ClaimStrength::Observed)
        .take(3)
        .map(|claim| {
            format!(
                "- {}",
                calibrate_content_claim(claim).assertion.render_sentence()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "## Summary\n\n- Reviewed {} repository entries and {} typed evidence facts.\n- Recorded {} route assessments and {} content slots.\n- Prepared {} dry-run patch operations; {} items remain held.\n\n## Evidence-backed observations\n\nThe audit emitted {observed_claims} observed claims. Examples:\n\n{examples}\n\n## Boundaries\n\nRepoSeiri did not write files, execute commands, call GitHub, create policy text, or establish popularity, trust, security, quality, or publication readiness.\n",
        summary.entries_scanned,
        summary.evidence_facts,
        summary.route_assessments,
        summary.route_content_slots,
        summary.patch_operations,
        summary.patch_holds,
    );
    CodexPrBody {
        title: "Organize repository routes with RepoSeiri".to_string(),
        body,
        draft: true,
    }
}

fn linter_boundary() -> &'static str {
    "Wording findings are evidence-scoped review hints, not legal, security, quality, trust, or publication-readiness judgments."
}

#[must_use]
pub fn render_query_markdown(view: &CodexQueryView<'_>) -> String {
    let mut out = format!(
        "# RepoSeiri Codex Query\n\n- Schema: `{}`\n- Repository: `{}`\n- Query: `{}`\n",
        view.schema_version,
        view.repo_root,
        query_kind(&view.query).slug(),
    );
    match &view.query {
        CodexQuery::Summary(summary) => {
            out.push_str(&format!(
                "\n- Entries: `{}`\n- Evidence facts: `{}`\n- Route assessments: `{}`\n- Content slots: `{}`\n- Findings: `{}`\n- Patch operations: `{}`\n- Patch holds: `{}`\n",
                summary.entries_scanned,
                summary.evidence_facts,
                summary.route_assessments,
                summary.route_content_slots,
                summary.findings,
                summary.patch_operations,
                summary.patch_holds,
            ));
        }
        CodexQuery::Routes(routes) => {
            out.push_str("\n## Routes\n");
            for assessment in routes.assessments {
                let state = assessment.summary_projection();
                out.push_str(&format!(
                    "- `{:?}`: `{:?}`; root={}, readme={}, inherited={}\n",
                    assessment.route(),
                    state.state,
                    assessment.presence().root_structured(),
                    assessment.readme().routing().is_present(),
                    assessment.presence().inherited(),
                ));
            }
        }
        CodexQuery::Evidence(evidence) => out.push_str(&format!(
            "\n- Documents: `{}`\n- Facts: `{}`\n",
            evidence.kernel.documents().len(),
            evidence.kernel.facts().len(),
        )),
        CodexQuery::Documents(documents) => out.push_str(&format!(
            "\n- Indexed documents: `{}`\n- Structured GitHub documents: `{}`\n",
            documents.index.entries().len(),
            documents.github.documents().len(),
        )),
        CodexQuery::Governance(governance) => {
            out.push_str(&format!(
                "\n- Facets: `{}`\n- Content slots: `{}`\n- Conflicts: `{}`\n- Claims: `{}`\n\n## Evidence-Backed Claims\n",
                governance.facets.facets.len(),
                governance.route_content.assessments.len(),
                governance.consistency.conflicts.len(),
                governance.claims.len(),
            ));
            for claim in governance.claims {
                let projection = calibrate_content_claim(claim);
                let boundaries = projection
                    .boundaries
                    .iter()
                    .map(|boundary| format!("`{boundary:?}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "- `{}`: {} Evidence: `{}`. Claim-local boundaries: {}.\n",
                    claim.id(),
                    projection.assertion.render_sentence(),
                    claim.evidence_ids().len(),
                    boundaries,
                ));
            }
        }
        CodexQuery::Patches(plan) => out.push_str(&format!(
            "\n- Dry-run operations: `{}`\n- Held items: `{}`\n- Writes files: `{}`\n",
            plan.operations.len(),
            plan.held.len(),
            plan.writes_files,
        )),
        CodexQuery::Linter(linter) => out.push_str(&format!(
            "\n- Available: `{}`\n- Findings: `{}`\n",
            linter.report.is_some(),
            linter.report.map_or(0, |report| report.findings.len()),
        )),
        CodexQuery::Actions(actions) => {
            out.push_str("\n## Review Commands\n");
            for action in actions {
                out.push_str(&format!(
                    "- `{}` {:?}\n",
                    action.command.program(),
                    action.command.args(),
                ));
            }
        }
        CodexQuery::Remote(remote) => {
            out.push_str(&format!("\n- Remote status: `{:?}`\n", remote.status));
        }
        CodexQuery::PrBody(pr) => {
            out.push('\n');
            out.push_str(&pr.body);
        }
    }
    out.push_str(&format!("\n- Boundary: {}\n", view.boundary));
    out
}

const fn query_kind(query: &CodexQuery<'_>) -> CodexQueryKind {
    match query {
        CodexQuery::Summary(_) => CodexQueryKind::Summary,
        CodexQuery::Routes(_) => CodexQueryKind::Routes,
        CodexQuery::Evidence(_) => CodexQueryKind::Evidence,
        CodexQuery::Documents(_) => CodexQueryKind::Documents,
        CodexQuery::Governance(_) => CodexQueryKind::Governance,
        CodexQuery::Patches(_) => CodexQueryKind::Patches,
        CodexQuery::Linter(_) => CodexQueryKind::Linter,
        CodexQuery::Actions(_) => CodexQueryKind::Actions,
        CodexQuery::Remote(_) => CodexQueryKind::Remote,
        CodexQuery::PrBody(_) => CodexQueryKind::PrBody,
    }
}
