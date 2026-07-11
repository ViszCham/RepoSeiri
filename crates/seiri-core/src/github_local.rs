use crate::{
    CoverageIncompleteReason, CoverageStatus, DocumentId, ImportantFile, ImportantFileKind,
    RepositoryScopeGraph, ScopeNodeId, ScopeNodeKind, SourceSpan,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubDocumentKind {
    IssueForm,
    Workflow,
    DependencyBot,
    Codeowners,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredBudgetKind {
    SourceBytes,
    Nodes,
    Depth,
    ScalarBytes,
    Diagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubParseStatus {
    Parsed,
    ParsedPartial,
    InvalidUtf8,
    UnsupportedSyntax,
    Malformed,
    BudgetExceeded(StructuredBudgetKind),
    PermissionDenied,
}

impl GithubParseStatus {
    #[must_use]
    pub const fn coverage_status(self) -> CoverageStatus {
        match self {
            Self::Parsed => CoverageStatus::Complete,
            Self::ParsedPartial => {
                CoverageStatus::Partial(CoverageIncompleteReason::UnsupportedSyntax)
            }
            Self::InvalidUtf8 => CoverageStatus::Partial(CoverageIncompleteReason::InvalidUtf8),
            Self::UnsupportedSyntax => {
                CoverageStatus::Partial(CoverageIncompleteReason::UnsupportedSyntax)
            }
            Self::Malformed => CoverageStatus::Partial(CoverageIncompleteReason::ParseFailed),
            Self::BudgetExceeded(_) => {
                CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
            }
            Self::PermissionDenied => {
                CoverageStatus::Partial(CoverageIncompleteReason::PermissionDenied)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubDiagnosticKind {
    UnsupportedSyntax,
    MissingRequiredField,
    MalformedValue,
    MissingCodeowner,
    BudgetExceeded(StructuredBudgetKind),
    InvalidUtf8,
    PermissionDenied,
    UnknownField,
    UnsupportedCodeownersPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticUnknownReason {
    Omitted,
    ParseFailed,
    Inherited,
    CoveragePartial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum StaticValue<T> {
    Literal(T),
    Expression { span: SourceSpan },
    Unsupported { span: SourceSpan },
    Unknown(StaticUnknownReason),
}

impl<T> StaticValue<T> {
    #[must_use]
    pub const fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPermission {
    None,
    Read,
    Write,
    DefaultOrInheritedUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionEntry {
    pub scope: String,
    pub permission: StaticValue<TokenPermission>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionSet {
    pub default: TokenPermission,
    pub entries: Vec<PermissionEntry>,
    pub span: Option<SourceSpan>,
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            default: TokenPermission::DefaultOrInheritedUnknown,
            entries: Vec::new(),
            span: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubDiagnostic {
    pub kind: GithubDiagnosticKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueFormFieldKind {
    Input,
    Textarea,
    Dropdown,
    Checkboxes,
    Markdown,
    Upload,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueFormField {
    pub kind: IssueFormFieldKind,
    pub id: Option<String>,
    pub required: Option<bool>,
    pub span: SourceSpan,
    pub unknown_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueFormRequiredFields {
    pub name: bool,
    pub description: bool,
    pub body: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueRouteCandidateKind {
    Security,
    Question,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRouteCandidate {
    pub kind: IssueRouteCandidateKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueForm {
    pub name: Option<String>,
    pub description: Option<String>,
    pub fields: Vec<IssueFormField>,
    pub required_fields: IssueFormRequiredFields,
    pub unknown_top_level_keys: Vec<String>,
    pub route_candidates: Vec<IssueRouteCandidate>,
    pub schema: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub name: String,
    pub value: StaticValue<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ActionReferenceKind {
    LocalPath(String),
    Docker(String),
    FullObjectId(String),
    TagOrBranch(String),
    Dynamic,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReference {
    pub raw: StaticValue<String>,
    pub kind: ActionReferenceKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: Option<StaticValue<String>>,
    pub uses: Option<ActionReference>,
    pub has_run_script: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowJobCandidateKind {
    Test,
    Build,
    Lint,
    Documentation,
    Release,
    Security,
    Deploy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowJobCandidate {
    pub kind: WorkflowJobCandidateKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub id: String,
    pub span: SourceSpan,
    pub name: Option<StaticValue<String>>,
    pub permissions: PermissionSet,
    pub reusable_workflow: Option<ActionReference>,
    pub steps: Vec<WorkflowStep>,
    pub candidates: Vec<WorkflowJobCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    pub name: Option<String>,
    pub triggers: Vec<WorkflowTrigger>,
    pub jobs: Vec<WorkflowJob>,
    pub permissions: PermissionSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyBotProvider {
    Dependabot,
    Renovate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyUpdate {
    pub ecosystem: Option<String>,
    pub directory: Option<String>,
    pub schedule: Option<String>,
    pub span: SourceSpan,
    pub ecosystem_value: StaticValue<String>,
    pub directory_values: Vec<StaticValue<String>>,
    pub schedule_value: StaticValue<String>,
    pub open_pull_requests_limit: StaticValue<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyBot {
    pub provider: DependencyBotProvider,
    pub updates: Vec<DependencyUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeownerEntry {
    pub pattern: String,
    pub owners: Vec<String>,
    pub span: SourceSpan,
    pub program: CodeownersPatternProgram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", content = "value", rename_all = "snake_case")]
pub enum CodeownersOp {
    Root,
    Slash,
    Literal(String),
    Star,
    DoubleStar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeownersPatternProgram {
    pub ops: Vec<CodeownersOp>,
    pub owners: Vec<String>,
    pub source: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeownersSkippedLine {
    pub pattern: String,
    pub span: SourceSpan,
    pub diagnostic: GithubDiagnosticKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Codeowners {
    pub entries: Vec<CodeownerEntry>,
    pub skipped: Vec<CodeownersSkippedLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticalPathKind {
    Workflow,
    Intake,
    Ownership,
    DependencyAutomation,
    Security,
    Documentation,
    Manifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticalPathCoverage {
    pub scope_node: ScopeNodeId,
    pub kind: CriticalPathKind,
    pub observed: u32,
    pub parsed: u32,
    pub coverage: CoverageStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubSemanticsReport {
    pub critical_paths: Vec<CriticalPathCoverage>,
}

impl GithubSemanticsReport {
    #[must_use]
    pub fn build(
        documents: &GithubLocalDocuments,
        graph: &RepositoryScopeGraph,
        important_files: &[ImportantFile],
    ) -> Self {
        let repository = graph
            .nodes
            .iter()
            .find(|node| node.kind == ScopeNodeKind::Repository)
            .map(|node| node.id)
            .unwrap_or(ScopeNodeId(1));
        let mut critical_paths = Vec::new();
        for (kind, document_kind) in [
            (CriticalPathKind::Workflow, GithubDocumentKind::Workflow),
            (CriticalPathKind::Intake, GithubDocumentKind::IssueForm),
            (CriticalPathKind::Ownership, GithubDocumentKind::Codeowners),
            (
                CriticalPathKind::DependencyAutomation,
                GithubDocumentKind::DependencyBot,
            ),
        ] {
            let matching = documents
                .documents()
                .iter()
                .filter(|document| document.kind == document_kind)
                .collect::<Vec<_>>();
            critical_paths.push(CriticalPathCoverage {
                scope_node: repository,
                kind,
                observed: matching.len() as u32,
                parsed: matching
                    .iter()
                    .filter(|document| {
                        matches!(
                            document.status,
                            GithubParseStatus::Parsed | GithubParseStatus::ParsedPartial
                        )
                    })
                    .count() as u32,
                coverage: matching
                    .iter()
                    .map(|document| document.status.coverage_status())
                    .find(|status| *status != CoverageStatus::Complete)
                    .unwrap_or(CoverageStatus::Complete),
            });
        }
        let security_files = important_files
            .iter()
            .filter(|file| file.kind == ImportantFileKind::Security)
            .collect::<Vec<_>>();
        if security_files.is_empty() {
            critical_paths.push(simple_coverage(repository, CriticalPathKind::Security, 0));
        } else {
            for file in security_files {
                critical_paths.push(simple_coverage(
                    containing_scope(graph, &file.path).unwrap_or(repository),
                    CriticalPathKind::Security,
                    1,
                ));
            }
        }
        let docs = graph
            .nodes
            .iter()
            .filter(|node| node.kind == ScopeNodeKind::Documentation)
            .collect::<Vec<_>>();
        if docs.is_empty() {
            critical_paths.push(simple_coverage(
                repository,
                CriticalPathKind::Documentation,
                0,
            ));
        } else {
            for node in docs {
                critical_paths.push(simple_coverage(
                    containing_scope(graph, &node.path).unwrap_or(repository),
                    CriticalPathKind::Documentation,
                    1,
                ));
            }
        }
        if graph.manifests.is_empty() {
            critical_paths.push(CriticalPathCoverage {
                scope_node: repository,
                kind: CriticalPathKind::Manifest,
                observed: 0,
                parsed: 0,
                coverage: graph.manifest_coverage,
            });
        } else {
            for manifest in &graph.manifests {
                critical_paths.push(CriticalPathCoverage {
                    scope_node: containing_scope(graph, &manifest.path).unwrap_or(repository),
                    kind: CriticalPathKind::Manifest,
                    observed: 1,
                    parsed: u32::from(manifest.status == crate::ManifestObservationStatus::Parsed),
                    coverage: match manifest.status {
                        crate::ManifestObservationStatus::Parsed => CoverageStatus::Complete,
                        crate::ManifestObservationStatus::SourceTooLarge => {
                            CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
                        }
                        crate::ManifestObservationStatus::InvalidUtf8 => {
                            CoverageStatus::Partial(CoverageIncompleteReason::InvalidUtf8)
                        }
                        crate::ManifestObservationStatus::Malformed
                        | crate::ManifestObservationStatus::Unsupported => {
                            CoverageStatus::Partial(CoverageIncompleteReason::ParseFailed)
                        }
                    },
                });
            }
        }
        critical_paths.sort_by_key(|coverage| (coverage.kind, coverage.scope_node));
        Self { critical_paths }
    }
}

fn containing_scope(graph: &RepositoryScopeGraph, path: &str) -> Option<ScopeNodeId> {
    graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                ScopeNodeKind::Repository | ScopeNodeKind::Workspace | ScopeNodeKind::Package
            ) && (node.path.is_empty()
                || path == node.path
                || path
                    .strip_prefix(&node.path)
                    .is_some_and(|suffix| suffix.starts_with('/')))
        })
        .max_by_key(|node| node.path.len())
        .map(|node| node.id)
}

fn simple_coverage(
    scope_node: ScopeNodeId,
    kind: CriticalPathKind,
    observed: u32,
) -> CriticalPathCoverage {
    CriticalPathCoverage {
        scope_node,
        kind,
        observed,
        parsed: observed,
        coverage: CoverageStatus::Complete,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum GithubDocumentIr {
    IssueForm(IssueForm),
    Workflow(Workflow),
    DependencyBot(DependencyBot),
    Codeowners(Codeowners),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubLocalDocument {
    pub document_id: DocumentId,
    pub path: String,
    pub kind: GithubDocumentKind,
    pub status: GithubParseStatus,
    pub diagnostics: Vec<GithubDiagnostic>,
    pub ir: Option<GithubDocumentIr>,
}

impl GithubLocalDocument {
    pub fn try_new(
        document_id: DocumentId,
        path: String,
        kind: GithubDocumentKind,
        status: GithubParseStatus,
        diagnostics: Vec<GithubDiagnostic>,
        ir: Option<GithubDocumentIr>,
    ) -> Result<Self, GithubLocalDocumentError> {
        if path.is_empty() {
            return Err(GithubLocalDocumentError::EmptyPath);
        }
        if matches!(
            status,
            GithubParseStatus::Parsed | GithubParseStatus::ParsedPartial
        ) != ir.is_some()
        {
            return Err(GithubLocalDocumentError::StatusIrMismatch { path });
        }
        Ok(Self {
            document_id,
            path,
            kind,
            status,
            diagnostics,
            ir,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubLocalDocuments {
    documents: Vec<GithubLocalDocument>,
}

impl GithubLocalDocuments {
    pub fn try_new(documents: Vec<GithubLocalDocument>) -> Result<Self, GithubLocalDocumentsError> {
        let mut paths = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut previous_path = None;
        for document in &documents {
            if !paths.insert(document.path.as_str()) {
                return Err(GithubLocalDocumentsError::DuplicatePath(
                    document.path.clone(),
                ));
            }
            if !ids.insert(document.document_id) {
                return Err(GithubLocalDocumentsError::DuplicateDocumentId(
                    document.document_id,
                ));
            }
            if previous_path.is_some_and(|previous: &str| previous > document.path.as_str()) {
                return Err(GithubLocalDocumentsError::NonCanonicalOrder);
            }
            previous_path = Some(document.path.as_str());
        }
        Ok(Self { documents })
    }

    #[must_use]
    pub fn documents(&self) -> &[GithubLocalDocument] {
        &self.documents
    }

    #[must_use]
    pub fn coverage_status(&self, repository_status: CoverageStatus) -> CoverageStatus {
        if repository_status != CoverageStatus::Complete {
            return repository_status;
        }
        self.documents
            .iter()
            .map(|document| document.status.coverage_status())
            .find(|status| *status != CoverageStatus::Complete)
            .unwrap_or(CoverageStatus::Complete)
    }

    #[must_use]
    pub fn status_for_document(&self, document_id: DocumentId) -> Option<CoverageStatus> {
        self.documents
            .iter()
            .find(|document| document.document_id == document_id)
            .map(|document| document.status.coverage_status())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubLocalDocumentError {
    EmptyPath,
    StatusIrMismatch { path: String },
}

impl Display for GithubLocalDocumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("GitHub local document path must not be empty"),
            Self::StatusIrMismatch { path } => {
                write!(
                    formatter,
                    "GitHub local document status/IR mismatch for '{path}'"
                )
            }
        }
    }
}

impl std::error::Error for GithubLocalDocumentError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubLocalDocumentsError {
    DuplicatePath(String),
    DuplicateDocumentId(DocumentId),
    NonCanonicalOrder,
}

impl Display for GithubLocalDocumentsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePath(path) => write!(formatter, "duplicate GitHub local path '{path}'"),
            Self::DuplicateDocumentId(id) => {
                write!(formatter, "duplicate GitHub document id {id:?}")
            }
            Self::NonCanonicalOrder => {
                formatter.write_str("GitHub local documents must be sorted by path")
            }
        }
    }
}

impl std::error::Error for GithubLocalDocumentsError {}
