#![forbid(unsafe_code)]

use seiri_core::{
    DocumentIndex, DocumentRole, DocumentScanStatus, GithubDiagnostic, GithubDiagnosticKind,
    GithubDocumentIr, GithubDocumentKind, GithubLocalDocument, GithubLocalDocumentError,
    GithubLocalDocuments, GithubLocalDocumentsError, GithubParseStatus, IndexedDocument,
    SourceSpan, SourceStore, StructuredBudgetKind,
};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

mod codeowners;
mod renovate;
mod yaml;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredParseOptions {
    pub max_source_bytes: usize,
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_scalar_bytes: usize,
    pub max_diagnostics: usize,
}

impl Default for StructuredParseOptions {
    fn default() -> Self {
        Self {
            max_source_bytes: 512 * 1024,
            max_nodes: 4_096,
            max_depth: 32,
            max_scalar_bytes: 8 * 1024,
            max_diagnostics: 128,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubLocalParserError {
    MissingDocumentId { path: String },
    Document(GithubLocalDocumentError),
    Documents(GithubLocalDocumentsError),
}

impl Display for GithubLocalParserError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDocumentId { path } => {
                write!(
                    formatter,
                    "GitHub configuration '{path}' has no document id"
                )
            }
            Self::Document(error) => write!(formatter, "{error}"),
            Self::Documents(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for GithubLocalParserError {}

pub fn parse_repository_github_documents(
    repo_root: impl AsRef<Path>,
    document_index: &DocumentIndex,
) -> Result<GithubLocalDocuments, GithubLocalParserError> {
    parse_repository_github_documents_with_options(
        repo_root,
        document_index,
        &StructuredParseOptions::default(),
    )
}

pub fn parse_repository_github_documents_with_options(
    repo_root: impl AsRef<Path>,
    document_index: &DocumentIndex,
    options: &StructuredParseOptions,
) -> Result<GithubLocalDocuments, GithubLocalParserError> {
    let repo_root = repo_root.as_ref();
    parse_documents(document_index, |entry, kind| {
        parse_entry(repo_root, entry.path.as_str(), entry.status, kind, options)
    })
}

pub fn parse_repository_github_documents_from_source_store(
    document_index: &DocumentIndex,
    sources: &SourceStore,
) -> Result<GithubLocalDocuments, GithubLocalParserError> {
    parse_repository_github_documents_from_source_store_with_options(
        document_index,
        sources,
        &StructuredParseOptions::default(),
    )
}

pub fn parse_repository_github_documents_from_source_store_with_options(
    document_index: &DocumentIndex,
    sources: &SourceStore,
    options: &StructuredParseOptions,
) -> Result<GithubLocalDocuments, GithubLocalParserError> {
    parse_documents(document_index, |entry, kind| {
        let Some(source) = sources.get(&entry.path) else {
            return ParseOutcome::failed(
                status_from_document_status(entry.status),
                GithubDiagnostic {
                    kind: diagnostic_from_document_status(entry.status),
                    span: empty_span(),
                },
            );
        };
        parse_source_entry(
            source.bytes(),
            entry.path.as_str(),
            entry.status,
            kind,
            options,
        )
    })
}

fn parse_documents(
    document_index: &DocumentIndex,
    mut parse: impl FnMut(&IndexedDocument, GithubDocumentKind) -> ParseOutcome,
) -> Result<GithubLocalDocuments, GithubLocalParserError> {
    let mut documents = Vec::new();
    for entry in document_index
        .entries()
        .iter()
        .filter(|entry| entry.role == DocumentRole::GithubConfiguration)
    {
        let Some(kind) = github_document_kind(&entry.path) else {
            continue;
        };
        let document_id =
            entry
                .document_id
                .ok_or_else(|| GithubLocalParserError::MissingDocumentId {
                    path: entry.path.clone(),
                })?;
        let outcome = parse(entry, kind);
        documents.push(
            GithubLocalDocument::try_new(
                document_id,
                entry.path.clone(),
                kind,
                outcome.status,
                outcome.diagnostics,
                outcome.ir,
            )
            .map_err(GithubLocalParserError::Document)?,
        );
    }
    documents.sort_by(|left, right| left.path.cmp(&right.path));
    GithubLocalDocuments::try_new(documents).map_err(GithubLocalParserError::Documents)
}

pub fn github_document_kind(path: &str) -> Option<GithubDocumentKind> {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    if lower.starts_with(".github/issue_template/")
        && !lower.ends_with("/config.yml")
        && !lower.ends_with("/config.yaml")
        && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
    {
        Some(GithubDocumentKind::IssueForm)
    } else if lower.starts_with(".github/workflows/")
        && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
    {
        Some(GithubDocumentKind::Workflow)
    } else if matches!(
        lower.as_str(),
        ".github/dependabot.yml"
            | ".github/dependabot.yaml"
            | "renovate.json"
            | ".github/renovate.json"
            | ".renovaterc"
            | ".renovaterc.json"
    ) {
        Some(GithubDocumentKind::DependencyBot)
    } else if matches!(
        lower.as_str(),
        "codeowners" | ".github/codeowners" | "docs/codeowners"
    ) {
        Some(GithubDocumentKind::Codeowners)
    } else {
        None
    }
}

struct ParseOutcome {
    status: GithubParseStatus,
    diagnostics: Vec<GithubDiagnostic>,
    ir: Option<GithubDocumentIr>,
}

impl ParseOutcome {
    fn parsed(ir: GithubDocumentIr, diagnostics: Vec<GithubDiagnostic>) -> Self {
        Self {
            status: if diagnostics.is_empty() {
                GithubParseStatus::Parsed
            } else {
                GithubParseStatus::ParsedPartial
            },
            diagnostics,
            ir: Some(ir),
        }
    }

    fn failed(status: GithubParseStatus, diagnostic: GithubDiagnostic) -> Self {
        Self {
            status,
            diagnostics: vec![diagnostic],
            ir: None,
        }
    }
}

fn parse_entry(
    repo_root: &Path,
    path: &str,
    document_status: DocumentScanStatus,
    kind: GithubDocumentKind,
    options: &StructuredParseOptions,
) -> ParseOutcome {
    if document_status != DocumentScanStatus::NotMarkdown {
        return ParseOutcome::failed(
            status_from_document_status(document_status),
            GithubDiagnostic {
                kind: diagnostic_from_document_status(document_status),
                span: empty_span(),
            },
        );
    }

    let full_path = repo_root.join(path);
    let mut bytes = Vec::new();
    let bytes = match fs::File::open(&full_path).and_then(|handle| {
        handle
            .take(options.max_source_bytes.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    }) {
        Ok(bytes) => bytes,
        Err(error) => {
            let status = if error.kind() == io::ErrorKind::PermissionDenied {
                GithubParseStatus::PermissionDenied
            } else {
                GithubParseStatus::Malformed
            };
            let diagnostic = if error.kind() == io::ErrorKind::PermissionDenied {
                GithubDiagnosticKind::PermissionDenied
            } else {
                GithubDiagnosticKind::MalformedValue
            };
            return ParseOutcome::failed(
                status,
                GithubDiagnostic {
                    kind: diagnostic,
                    span: empty_span(),
                },
            );
        }
    };
    if bytes.len() > options.max_source_bytes {
        return ParseOutcome::failed(
            GithubParseStatus::BudgetExceeded(StructuredBudgetKind::SourceBytes),
            GithubDiagnostic {
                kind: GithubDiagnosticKind::BudgetExceeded(StructuredBudgetKind::SourceBytes),
                span: empty_span(),
            },
        );
    }
    parse_source_entry(&bytes, path, document_status, kind, options)
}

fn parse_source_entry(
    bytes: &[u8],
    path: &str,
    document_status: DocumentScanStatus,
    kind: GithubDocumentKind,
    options: &StructuredParseOptions,
) -> ParseOutcome {
    if document_status != DocumentScanStatus::NotMarkdown {
        return ParseOutcome::failed(
            status_from_document_status(document_status),
            GithubDiagnostic {
                kind: diagnostic_from_document_status(document_status),
                span: empty_span(),
            },
        );
    }
    if bytes.len() > options.max_source_bytes {
        return ParseOutcome::failed(
            GithubParseStatus::BudgetExceeded(StructuredBudgetKind::SourceBytes),
            GithubDiagnostic {
                kind: GithubDiagnosticKind::BudgetExceeded(StructuredBudgetKind::SourceBytes),
                span: empty_span(),
            },
        );
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return ParseOutcome::failed(
                GithubParseStatus::InvalidUtf8,
                GithubDiagnostic {
                    kind: GithubDiagnosticKind::InvalidUtf8,
                    span: SourceSpan::new(1, 1, 0, error.valid_up_to()),
                },
            );
        }
    };

    let result = match kind {
        GithubDocumentKind::IssueForm => yaml::parse_issue_form(text, options)
            .map(|(ir, diagnostics)| (GithubDocumentIr::IssueForm(ir), diagnostics)),
        GithubDocumentKind::Workflow => yaml::parse_workflow(text, options)
            .map(|(ir, diagnostics)| (GithubDocumentIr::Workflow(ir), diagnostics)),
        GithubDocumentKind::DependencyBot if is_renovate_path(path) => {
            renovate::parse_renovate(text, options)
                .map(|(ir, diagnostics)| (GithubDocumentIr::DependencyBot(ir), diagnostics))
        }
        GithubDocumentKind::DependencyBot => yaml::parse_dependabot(text, options)
            .map(|(ir, diagnostics)| (GithubDocumentIr::DependencyBot(ir), diagnostics)),
        GithubDocumentKind::Codeowners => codeowners::parse_codeowners(text, options)
            .map(|(ir, diagnostics)| (GithubDocumentIr::Codeowners(ir), diagnostics)),
    };
    match result {
        Ok((ir, diagnostics)) if diagnostics.len() <= options.max_diagnostics => {
            ParseOutcome::parsed(ir, diagnostics)
        }
        Ok(_) | Err(_) if options.max_diagnostics == 0 => ParseOutcome::failed(
            GithubParseStatus::BudgetExceeded(StructuredBudgetKind::Diagnostics),
            GithubDiagnostic {
                kind: GithubDiagnosticKind::BudgetExceeded(StructuredBudgetKind::Diagnostics),
                span: empty_span(),
            },
        ),
        Err(failure) => ParseOutcome::failed(failure.status, failure.diagnostic),
        Ok(_) => ParseOutcome::failed(
            GithubParseStatus::BudgetExceeded(StructuredBudgetKind::Diagnostics),
            GithubDiagnostic {
                kind: GithubDiagnosticKind::BudgetExceeded(StructuredBudgetKind::Diagnostics),
                span: empty_span(),
            },
        ),
    }
}

fn is_renovate_path(path: &str) -> bool {
    matches!(
        path.to_ascii_lowercase().as_str(),
        "renovate.json" | ".github/renovate.json" | ".renovaterc" | ".renovaterc.json"
    )
}

fn status_from_document_status(status: DocumentScanStatus) -> GithubParseStatus {
    match status {
        DocumentScanStatus::InvalidUtf8 => GithubParseStatus::InvalidUtf8,
        DocumentScanStatus::SkippedDocumentBudget | DocumentScanStatus::SkippedByteBudget => {
            GithubParseStatus::BudgetExceeded(StructuredBudgetKind::SourceBytes)
        }
        DocumentScanStatus::PermissionDenied => GithubParseStatus::PermissionDenied,
        DocumentScanStatus::NotMarkdown
        | DocumentScanStatus::Scanned
        | DocumentScanStatus::ParseFailed => GithubParseStatus::Malformed,
    }
}

fn diagnostic_from_document_status(status: DocumentScanStatus) -> GithubDiagnosticKind {
    match status {
        DocumentScanStatus::InvalidUtf8 => GithubDiagnosticKind::InvalidUtf8,
        DocumentScanStatus::SkippedDocumentBudget | DocumentScanStatus::SkippedByteBudget => {
            GithubDiagnosticKind::BudgetExceeded(StructuredBudgetKind::SourceBytes)
        }
        DocumentScanStatus::PermissionDenied => GithubDiagnosticKind::PermissionDenied,
        DocumentScanStatus::NotMarkdown
        | DocumentScanStatus::Scanned
        | DocumentScanStatus::ParseFailed => GithubDiagnosticKind::MalformedValue,
    }
}

const fn empty_span() -> SourceSpan {
    SourceSpan::new(1, 1, 0, 0)
}

pub(crate) struct ParseFailure {
    pub status: GithubParseStatus,
    pub diagnostic: GithubDiagnostic,
}

pub(crate) fn failure(
    status: GithubParseStatus,
    kind: GithubDiagnosticKind,
    span: SourceSpan,
) -> ParseFailure {
    ParseFailure {
        status,
        diagnostic: GithubDiagnostic { kind, span },
    }
}

pub(crate) fn limit_failure(kind: StructuredBudgetKind, span: SourceSpan) -> ParseFailure {
    failure(
        GithubParseStatus::BudgetExceeded(kind),
        GithubDiagnosticKind::BudgetExceeded(kind),
        span,
    )
}

pub(crate) fn unsupported_failure(span: SourceSpan) -> ParseFailure {
    failure(
        GithubParseStatus::UnsupportedSyntax,
        GithubDiagnosticKind::UnsupportedSyntax,
        span,
    )
}

pub(crate) fn malformed_failure(kind: GithubDiagnosticKind, span: SourceSpan) -> ParseFailure {
    failure(GithubParseStatus::Malformed, kind, span)
}

pub(crate) fn scalar_within_limit(
    value: &str,
    options: &StructuredParseOptions,
    span: SourceSpan,
) -> Result<(), ParseFailure> {
    if value.len() > options.max_scalar_bytes {
        Err(limit_failure(StructuredBudgetKind::ScalarBytes, span))
    } else {
        Ok(())
    }
}

pub(crate) fn check_diagnostics(
    diagnostics: &[GithubDiagnostic],
    options: &StructuredParseOptions,
    span: SourceSpan,
) -> Result<(), ParseFailure> {
    if diagnostics.len() > options.max_diagnostics {
        Err(limit_failure(StructuredBudgetKind::Diagnostics, span))
    } else {
        Ok(())
    }
}
