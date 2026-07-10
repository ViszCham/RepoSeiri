use crate::{CoverageIncompleteReason, CoverageStatus, DocumentId, SourceSpan};
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
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueFormField {
    pub kind: IssueFormFieldKind,
    pub id: Option<String>,
    pub required: Option<bool>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueForm {
    pub name: Option<String>,
    pub description: Option<String>,
    pub fields: Vec<IssueFormField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub id: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    pub name: Option<String>,
    pub triggers: Vec<WorkflowTrigger>,
    pub jobs: Vec<WorkflowJob>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Codeowners {
    pub entries: Vec<CodeownerEntry>,
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
        if (status == GithubParseStatus::Parsed) != ir.is_some() {
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
