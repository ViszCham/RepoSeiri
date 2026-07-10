use crate::{EvidenceConfidence, EvidenceKind, EvidenceScope, RouteKind, RouteSource, SourceSpan};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceScanner {
    FileSystem,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceEvent {
    ImportantFileDetection,
    ReadmeDiscovery,
    MarkdownHeading,
    MarkdownLink,
    MarkdownBadge,
    RouteCandidate { source: RouteSource },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceOrigin {
    pub scanner: EvidenceScanner,
    pub event: EvidenceEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceDraft {
    pub kind: EvidenceKind,
    pub path: Option<String>,
    pub route: Option<RouteKind>,
    pub value: String,
    pub origin: EvidenceOrigin,
    pub scope: EvidenceScope,
    pub confidence: EvidenceConfidence,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKernelError {
    OriginKindMismatch {
        kind: EvidenceKind,
        origin: EvidenceOrigin,
    },
    MissingSourceSpan {
        kind: EvidenceKind,
    },
}

impl Display for EvidenceKernelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OriginKindMismatch { kind, origin } => {
                write!(
                    formatter,
                    "evidence kind {kind:?} does not match origin {origin:?}"
                )
            }
            Self::MissingSourceSpan { kind } => {
                write!(
                    formatter,
                    "source-backed evidence kind {kind:?} requires a span"
                )
            }
        }
    }
}

impl std::error::Error for EvidenceKernelError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFact {
    pub id: EvidenceId,
    pub kind: EvidenceKind,
    pub path: Option<String>,
    pub route: Option<RouteKind>,
    pub value: String,
    pub origin: EvidenceOrigin,
    pub scope: EvidenceScope,
    pub confidence: EvidenceConfidence,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EvidenceKernel {
    facts: Vec<EvidenceFact>,
}

impl EvidenceKernel {
    pub fn from_drafts(drafts: Vec<EvidenceDraft>) -> Result<Self, EvidenceKernelError> {
        for draft in &drafts {
            validate_fact_shape(draft.kind, draft.origin, draft.span)?;
        }
        let facts = drafts
            .into_iter()
            .enumerate()
            .map(|(index, draft)| EvidenceFact {
                id: stable_evidence_id(index + 1),
                kind: draft.kind,
                path: draft.path,
                route: draft.route,
                value: draft.value,
                origin: draft.origin,
                scope: draft.scope,
                confidence: draft.confidence,
                span: draft.span,
            })
            .collect();
        Ok(Self { facts })
    }

    #[must_use]
    pub fn facts(&self) -> &[EvidenceFact] {
        &self.facts
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
            facts: Vec<EvidenceFact>,
        }

        let wire = WireKernel::deserialize(deserializer)?;
        for (index, fact) in wire.facts.iter().enumerate() {
            if fact.id != stable_evidence_id(index + 1) {
                return Err(D::Error::custom(
                    "evidence facts must have contiguous deterministic ids in storage order",
                ));
            }
            validate_fact_shape(fact.kind, fact.origin, fact.span).map_err(D::Error::custom)?;
        }
        Ok(Self { facts: wire.facts })
    }
}

fn validate_fact_shape(
    kind: EvidenceKind,
    origin: EvidenceOrigin,
    span: Option<SourceSpan>,
) -> Result<(), EvidenceKernelError> {
    let origin_matches = matches!(
        (kind, origin.scanner, origin.event),
        (
            EvidenceKind::FilePresent | EvidenceKind::ImportantFile,
            EvidenceScanner::FileSystem,
            EvidenceEvent::ImportantFileDetection
        ) | (
            EvidenceKind::ReadmePresent | EvidenceKind::ReadmeMissing,
            EvidenceScanner::Markdown,
            EvidenceEvent::ReadmeDiscovery
        ) | (
            EvidenceKind::MarkdownHeading,
            EvidenceScanner::Markdown,
            EvidenceEvent::MarkdownHeading
        ) | (
            EvidenceKind::MarkdownLink,
            EvidenceScanner::Markdown,
            EvidenceEvent::MarkdownLink
        ) | (
            EvidenceKind::MarkdownBadge,
            EvidenceScanner::Markdown,
            EvidenceEvent::MarkdownBadge
        ) | (
            EvidenceKind::RouteCandidate,
            EvidenceScanner::Markdown,
            EvidenceEvent::RouteCandidate { .. }
        )
    );
    if !origin_matches {
        return Err(EvidenceKernelError::OriginKindMismatch { kind, origin });
    }
    if matches!(
        kind,
        EvidenceKind::MarkdownHeading
            | EvidenceKind::MarkdownLink
            | EvidenceKind::MarkdownBadge
            | EvidenceKind::RouteCandidate
    ) && span.is_none()
    {
        return Err(EvidenceKernelError::MissingSourceSpan { kind });
    }
    Ok(())
}
