use crate::{
    EvidenceConfidence, EvidenceId, EvidenceKernel, EvidenceKind, EvidenceScanner, EvidenceScope,
    ImportantFileKind, RouteKind, SourceSpan,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(NonZeroU32);

impl DocumentId {
    fn from_ordinal(ordinal: usize) -> Option<Self> {
        u32::try_from(ordinal)
            .ok()
            .and_then(NonZeroU32::new)
            .map(Self)
    }

    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ByteOffset(u32);

impl ByteOffset {
    pub fn try_from_usize(value: usize) -> Result<Self, EvidenceKernelV2Error> {
        u32::try_from(value)
            .map(Self)
            .map_err(|_| EvidenceKernelV2Error::OffsetOverflow { value })
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpanV2 {
    pub line: NonZeroU32,
    pub column: NonZeroU32,
    pub byte_start: ByteOffset,
    pub byte_end: ByteOffset,
}

impl TryFrom<SourceSpan> for SourceSpanV2 {
    type Error = EvidenceKernelV2Error;

    fn try_from(value: SourceSpan) -> Result<Self, Self::Error> {
        let line = u32::try_from(value.line)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(EvidenceKernelV2Error::LineOverflow { value: value.line })?;
        let column = u32::try_from(value.column)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(EvidenceKernelV2Error::ColumnOverflow {
                value: value.column,
            })?;
        Ok(Self {
            line,
            column,
            byte_start: ByteOffset::try_from_usize(value.byte_start)?,
            byte_end: ByteOffset::try_from_usize(value.byte_end)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDomain {
    RepositoryLocal,
    OrganizationInherited,
    RemoteRepository,
    Fixture,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceProducer {
    FileWalker,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadmePresence {
    Present,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownEvidenceKind {
    Heading,
    Link,
    Badge,
    RouteCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EvidenceAtom {
    FilePresent,
    ImportantFile(ImportantFileKind),
    Readme(ReadmePresence),
    Markdown {
        event: MarkdownEvidenceKind,
        route: Option<RouteKind>,
    },
}

impl EvidenceAtom {
    #[must_use]
    pub const fn route(self) -> Option<RouteKind> {
        match self {
            Self::ImportantFile(kind) => route_for_important_file(kind),
            Self::Readme(_) => Some(RouteKind::Identity),
            Self::Markdown { route, .. } => route,
            Self::FilePresent => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceProvenance {
    pub domain: SourceDomain,
    pub producer: EvidenceProducer,
    pub document: Option<DocumentId>,
    pub span: Option<SourceSpanV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: DocumentId,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFactV2 {
    pub id: EvidenceId,
    pub atom: EvidenceAtom,
    pub provenance: EvidenceProvenance,
    pub confidence: EvidenceConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceKernelV2 {
    documents: Vec<DocumentRecord>,
    facts: Vec<EvidenceFactV2>,
}

impl EvidenceKernelV2 {
    pub fn from_legacy(kernel: &EvidenceKernel) -> Result<Self, EvidenceKernelV2Error> {
        let paths = kernel
            .facts()
            .iter()
            .filter_map(|fact| fact.path.as_deref())
            .collect::<BTreeSet<_>>();
        let mut document_ids = BTreeMap::new();
        let mut documents = Vec::with_capacity(paths.len());
        for (index, path) in paths.into_iter().enumerate() {
            let id = DocumentId::from_ordinal(index + 1)
                .ok_or(EvidenceKernelV2Error::TooManyDocuments)?;
            document_ids.insert(path, id);
            documents.push(DocumentRecord {
                id,
                path: path.to_string(),
            });
        }

        let mut facts = Vec::with_capacity(kernel.len());
        for fact in kernel.facts() {
            let document = fact
                .path
                .as_deref()
                .and_then(|path| document_ids.get(path).copied());
            facts.push(EvidenceFactV2 {
                id: fact.id,
                atom: atom_from_legacy(fact.kind, fact.route, &fact.value)?,
                provenance: EvidenceProvenance {
                    domain: domain_from_scope(fact.scope),
                    producer: producer_from_scanner(fact.origin.scanner),
                    document,
                    span: fact.span.map(SourceSpanV2::try_from).transpose()?,
                },
                confidence: fact.confidence,
            });
        }
        Ok(Self { documents, facts })
    }

    #[must_use]
    pub fn documents(&self) -> &[DocumentRecord] {
        &self.documents
    }

    #[must_use]
    pub fn facts(&self) -> &[EvidenceFactV2] {
        &self.facts
    }

    #[must_use]
    pub fn document_id_for_path(&self, path: &str) -> Option<DocumentId> {
        self.documents
            .binary_search_by(|document| document.path.as_str().cmp(path))
            .ok()
            .map(|index| self.documents[index].id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceKernelV2Error {
    TooManyDocuments,
    OffsetOverflow { value: usize },
    LineOverflow { value: usize },
    ColumnOverflow { value: usize },
    UnknownImportantFile { value: String },
}

impl Display for EvidenceKernelV2Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyDocuments => formatter.write_str("document count exceeds non-zero u32"),
            Self::OffsetOverflow { value } => write!(formatter, "byte offset {value} exceeds u32"),
            Self::LineOverflow { value } => write!(formatter, "line {value} exceeds non-zero u32"),
            Self::ColumnOverflow { value } => {
                write!(formatter, "column {value} exceeds non-zero u32")
            }
            Self::UnknownImportantFile { value } => {
                write!(formatter, "unknown important-file value '{value}'")
            }
        }
    }
}

impl std::error::Error for EvidenceKernelV2Error {}

fn atom_from_legacy(
    kind: EvidenceKind,
    route: Option<RouteKind>,
    value: &str,
) -> Result<EvidenceAtom, EvidenceKernelV2Error> {
    Ok(match kind {
        EvidenceKind::FilePresent => EvidenceAtom::FilePresent,
        EvidenceKind::ImportantFile => {
            EvidenceAtom::ImportantFile(parse_important_file_kind(value).ok_or_else(|| {
                EvidenceKernelV2Error::UnknownImportantFile {
                    value: value.to_string(),
                }
            })?)
        }
        EvidenceKind::ReadmePresent => EvidenceAtom::Readme(ReadmePresence::Present),
        EvidenceKind::ReadmeMissing => EvidenceAtom::Readme(ReadmePresence::Absent),
        EvidenceKind::MarkdownHeading => EvidenceAtom::Markdown {
            event: MarkdownEvidenceKind::Heading,
            route,
        },
        EvidenceKind::MarkdownLink => EvidenceAtom::Markdown {
            event: MarkdownEvidenceKind::Link,
            route,
        },
        EvidenceKind::MarkdownBadge => EvidenceAtom::Markdown {
            event: MarkdownEvidenceKind::Badge,
            route,
        },
        EvidenceKind::RouteCandidate => EvidenceAtom::Markdown {
            event: MarkdownEvidenceKind::RouteCandidate,
            route,
        },
    })
}

const fn domain_from_scope(scope: EvidenceScope) -> SourceDomain {
    match scope {
        EvidenceScope::Root | EvidenceScope::Nested | EvidenceScope::Generated => {
            SourceDomain::RepositoryLocal
        }
        EvidenceScope::Fixture => SourceDomain::Fixture,
        EvidenceScope::Unknown => SourceDomain::Unknown,
    }
}

const fn producer_from_scanner(scanner: EvidenceScanner) -> EvidenceProducer {
    match scanner {
        EvidenceScanner::FileSystem => EvidenceProducer::FileWalker,
        EvidenceScanner::Markdown => EvidenceProducer::Markdown,
    }
}

fn parse_important_file_kind(value: &str) -> Option<ImportantFileKind> {
    Some(match value {
        "Readme" => ImportantFileKind::Readme,
        "License" => ImportantFileKind::License,
        "Contributing" => ImportantFileKind::Contributing,
        "Security" => ImportantFileKind::Security,
        "Support" => ImportantFileKind::Support,
        "IssueTemplate" => ImportantFileKind::IssueTemplate,
        "IssueForm" => ImportantFileKind::IssueForm,
        "PullRequestTemplate" => ImportantFileKind::PullRequestTemplate,
        "Changelog" => ImportantFileKind::Changelog,
        "Codeowners" => ImportantFileKind::Codeowners,
        "CargoToml" => ImportantFileKind::CargoToml,
        "DocsDirectory" => ImportantFileKind::DocsDirectory,
        "Workflow" => ImportantFileKind::Workflow,
        "DependencyBot" => ImportantFileKind::DependencyBot,
        "SecurityAutomation" => ImportantFileKind::SecurityAutomation,
        "Gitignore" => ImportantFileKind::Gitignore,
        "Gitattributes" => ImportantFileKind::Gitattributes,
        "EditorConfig" => ImportantFileKind::EditorConfig,
        _ => return None,
    })
}

const fn route_for_important_file(kind: ImportantFileKind) -> Option<RouteKind> {
    match kind {
        ImportantFileKind::Readme | ImportantFileKind::CargoToml => Some(RouteKind::Identity),
        ImportantFileKind::License => Some(RouteKind::License),
        ImportantFileKind::Contributing => Some(RouteKind::Contributing),
        ImportantFileKind::Security => Some(RouteKind::Security),
        ImportantFileKind::Support => Some(RouteKind::Support),
        ImportantFileKind::IssueTemplate
        | ImportantFileKind::IssueForm
        | ImportantFileKind::PullRequestTemplate => Some(RouteKind::Intake),
        ImportantFileKind::Changelog => Some(RouteKind::Release),
        ImportantFileKind::Codeowners => Some(RouteKind::Ownership),
        ImportantFileKind::DocsDirectory => Some(RouteKind::Docs),
        ImportantFileKind::Workflow
        | ImportantFileKind::DependencyBot
        | ImportantFileKind::SecurityAutomation => Some(RouteKind::Automation),
        ImportantFileKind::Gitignore
        | ImportantFileKind::Gitattributes
        | ImportantFileKind::EditorConfig => Some(RouteKind::Hygiene),
    }
}
