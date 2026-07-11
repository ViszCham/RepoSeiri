use crate::{EvidenceConfidence, ImportantFileKind, RouteKind, SourceSpan};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::num::NonZeroU32;
use std::str::FromStr;

const EVIDENCE_ID_PREFIX: &str = "evrec-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EvidenceId(NonZeroU32);

impl EvidenceId {
    #[must_use]
    pub fn from_ordinal(ordinal: usize) -> Option<Self> {
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

impl Display for EvidenceId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{EVIDENCE_ID_PREFIX}{:04}", self.ordinal())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseEvidenceIdError;

impl Display for ParseEvidenceIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("evidence id must use canonical evrec-NNNN form with a non-zero u32")
    }
}

impl std::error::Error for ParseEvidenceIdError {}

impl FromStr for EvidenceId {
    type Err = ParseEvidenceIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let ordinal = value
            .strip_prefix(EVIDENCE_ID_PREFIX)
            .ok_or(ParseEvidenceIdError)?
            .parse::<u32>()
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(ParseEvidenceIdError)?;
        let id = Self(ordinal);
        (id.to_string() == value)
            .then_some(id)
            .ok_or(ParseEvidenceIdError)
    }
}

impl Serialize for EvidenceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for EvidenceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(D::Error::custom)
    }
}

#[must_use]
pub fn stable_evidence_id(ordinal: usize) -> EvidenceId {
    EvidenceId::from_ordinal(ordinal).expect("evidence ordinal must fit a non-zero u32")
}

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
    fn try_from_usize(value: usize) -> Result<Self, EvidenceKernelError> {
        u32::try_from(value)
            .map(Self)
            .map_err(|_| EvidenceKernelError::OffsetOverflow { value })
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSourceSpan {
    pub line: NonZeroU32,
    pub column: NonZeroU32,
    pub byte_start: ByteOffset,
    pub byte_end: ByteOffset,
}

impl TryFrom<SourceSpan> for EvidenceSourceSpan {
    type Error = EvidenceKernelError;

    fn try_from(value: SourceSpan) -> Result<Self, Self::Error> {
        let line = u32::try_from(value.line)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(EvidenceKernelError::LineOverflow { value: value.line })?;
        let column = u32::try_from(value.column)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or(EvidenceKernelError::ColumnOverflow {
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

    #[must_use]
    pub const fn is_structural(self) -> bool {
        matches!(
            self,
            Self::FilePresent | Self::ImportantFile(_) | Self::Readme(ReadmePresence::Present)
        )
    }

    #[must_use]
    pub const fn is_markdown_route(self) -> bool {
        matches!(self, Self::Markdown { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceProvenance {
    pub domain: SourceDomain,
    pub producer: EvidenceProducer,
    pub document: Option<DocumentId>,
    pub span: Option<EvidenceSourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceDraft {
    pub atom: EvidenceAtom,
    pub domain: SourceDomain,
    pub producer: EvidenceProducer,
    pub path: Option<String>,
    pub span: Option<SourceSpan>,
    pub confidence: EvidenceConfidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub id: DocumentId,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFact {
    pub id: EvidenceId,
    pub atom: EvidenceAtom,
    pub provenance: EvidenceProvenance,
    pub confidence: EvidenceConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EvidenceKernel {
    documents: Vec<DocumentRecord>,
    facts: Vec<EvidenceFact>,
}

impl EvidenceKernel {
    pub fn from_drafts(drafts: Vec<EvidenceDraft>) -> Result<Self, EvidenceKernelError> {
        let paths = drafts
            .iter()
            .filter_map(|draft| draft.path.clone())
            .collect::<BTreeSet<_>>();
        let mut document_ids = BTreeMap::new();
        let mut documents = Vec::with_capacity(paths.len());
        for (index, path) in paths.into_iter().enumerate() {
            let id =
                DocumentId::from_ordinal(index + 1).ok_or(EvidenceKernelError::TooManyDocuments)?;
            document_ids.insert(path.clone(), id);
            documents.push(DocumentRecord { id, path });
        }

        let mut facts = Vec::with_capacity(drafts.len());
        for (index, draft) in drafts.into_iter().enumerate() {
            validate_draft(&draft)?;
            let document = draft
                .path
                .as_deref()
                .and_then(|path| document_ids.get(path).copied());
            facts.push(EvidenceFact {
                id: stable_evidence_id(index + 1),
                atom: draft.atom,
                provenance: EvidenceProvenance {
                    domain: draft.domain,
                    producer: draft.producer,
                    document,
                    span: draft.span.map(EvidenceSourceSpan::try_from).transpose()?,
                },
                confidence: draft.confidence,
            });
        }
        Ok(Self { documents, facts })
    }

    #[must_use]
    pub fn documents(&self) -> &[DocumentRecord] {
        &self.documents
    }

    #[must_use]
    pub fn facts(&self) -> &[EvidenceFact] {
        &self.facts
    }

    #[must_use]
    pub fn document_id_for_path(&self, path: &str) -> Option<DocumentId> {
        self.documents
            .binary_search_by(|document| document.path.as_str().cmp(path))
            .ok()
            .map(|index| self.documents[index].id)
    }

    #[must_use]
    pub fn path_for_document(&self, id: DocumentId) -> Option<&str> {
        self.documents
            .get(usize::try_from(id.ordinal()).ok()?.checked_sub(1)?)
            .filter(|document| document.id == id)
            .map(|document| document.path.as_str())
    }

    #[must_use]
    pub fn path_for_fact(&self, fact: &EvidenceFact) -> Option<&str> {
        fact.provenance
            .document
            .and_then(|document| self.path_for_document(document))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }
}

impl<'de> Deserialize<'de> for EvidenceKernel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireKernel {
            documents: Vec<DocumentRecord>,
            facts: Vec<EvidenceFact>,
        }

        let wire = WireKernel::deserialize(deserializer)?;
        for (index, document) in wire.documents.iter().enumerate() {
            if document.id
                != DocumentId::from_ordinal(index + 1)
                    .ok_or_else(|| D::Error::custom("document count exceeds non-zero u32"))?
            {
                return Err(D::Error::custom(
                    "documents must have contiguous deterministic ids in path order",
                ));
            }
            if index > 0 && wire.documents[index - 1].path >= document.path {
                return Err(D::Error::custom(
                    "documents must be strictly ordered by canonical path",
                ));
            }
        }
        for (index, fact) in wire.facts.iter().enumerate() {
            if fact.id != stable_evidence_id(index + 1) {
                return Err(D::Error::custom(
                    "evidence facts must have contiguous deterministic ids in storage order",
                ));
            }
            if let Some(document) = fact.provenance.document {
                let Some(record) = wire.documents.get(
                    usize::try_from(document.ordinal())
                        .unwrap_or(0)
                        .saturating_sub(1),
                ) else {
                    return Err(D::Error::custom("evidence references an unknown document"));
                };
                if record.id != document {
                    return Err(D::Error::custom("evidence document id is not canonical"));
                }
            }
            validate_fact(fact).map_err(D::Error::custom)?;
        }
        Ok(Self {
            documents: wire.documents,
            facts: wire.facts,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKernelError {
    TooManyDocuments,
    OffsetOverflow { value: usize },
    LineOverflow { value: usize },
    ColumnOverflow { value: usize },
    ProducerAtomMismatch,
    MissingSourceSpan,
    UnexpectedSourceSpan,
}

impl Display for EvidenceKernelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyDocuments => formatter.write_str("document count exceeds non-zero u32"),
            Self::OffsetOverflow { value } => write!(formatter, "byte offset {value} exceeds u32"),
            Self::LineOverflow { value } => write!(formatter, "line {value} exceeds non-zero u32"),
            Self::ColumnOverflow { value } => {
                write!(formatter, "column {value} exceeds non-zero u32")
            }
            Self::ProducerAtomMismatch => {
                formatter.write_str("evidence producer does not match the typed atom")
            }
            Self::MissingSourceSpan => {
                formatter.write_str("markdown evidence requires a checked source span")
            }
            Self::UnexpectedSourceSpan => {
                formatter.write_str("non-markdown evidence must not carry a source span")
            }
        }
    }
}

impl std::error::Error for EvidenceKernelError {}

fn validate_draft(draft: &EvidenceDraft) -> Result<(), EvidenceKernelError> {
    let fact = EvidenceFact {
        id: stable_evidence_id(1),
        atom: draft.atom,
        provenance: EvidenceProvenance {
            domain: draft.domain,
            producer: draft.producer,
            document: None,
            span: draft.span.map(EvidenceSourceSpan::try_from).transpose()?,
        },
        confidence: draft.confidence,
    };
    validate_fact(&fact)
}

fn validate_fact(fact: &EvidenceFact) -> Result<(), EvidenceKernelError> {
    let producer_matches = matches!(
        (fact.atom, fact.provenance.producer),
        (
            EvidenceAtom::FilePresent | EvidenceAtom::ImportantFile(_),
            EvidenceProducer::FileWalker
        ) | (
            EvidenceAtom::Readme(_) | EvidenceAtom::Markdown { .. },
            EvidenceProducer::Markdown
        )
    );
    if !producer_matches {
        return Err(EvidenceKernelError::ProducerAtomMismatch);
    }
    match (fact.atom, fact.provenance.span) {
        (EvidenceAtom::Markdown { .. }, None) => Err(EvidenceKernelError::MissingSourceSpan),
        (EvidenceAtom::Markdown { .. }, Some(_)) => Ok(()),
        (_, Some(_)) => Err(EvidenceKernelError::UnexpectedSourceSpan),
        (_, None) => Ok(()),
    }
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
