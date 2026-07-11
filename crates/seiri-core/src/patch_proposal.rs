use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

mod base;
mod binding;
mod engine;

use base::detect_line_ending;
pub use base::{PatchBaseDigest, TextDocumentBase, TextEditSpan, TextEncoding, TextLineEnding};
pub use binding::{
    PatchAnalysisRun, PatchAnchorContext, PatchAnchorSlice, PatchProposalBinding,
    PatchProposalBindingError, PATCH_ANCHOR_CONTEXT_BYTES,
};

pub const PATCH_PROPOSAL_SCHEMA_VERSION: &str = "seiri.patch-proposal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySlotKind {
    SecurityPolicy,
    SupportPolicy,
    LifecyclePolicy,
    LicenseDecision,
    OwnershipPolicy,
    ContactChannel,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedPolicySlot {
    pub id: String,
    pub kind: PolicySlotKind,
    pub required_decision: String,
}

impl UnresolvedPolicySlot {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        kind: PolicySlotKind,
        required_decision: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            required_decision: required_decision.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum PatchEditContent {
    Literal(String),
    UnresolvedSlot(UnresolvedPolicySlot),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchTextEdit {
    pub id: String,
    pub span: TextEditSpan,
    pub content: PatchEditContent,
}

impl PatchTextEdit {
    #[must_use]
    pub fn literal(
        id: impl Into<String>,
        span: TextEditSpan,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            span,
            content: PatchEditContent::Literal(replacement.into()),
        }
    }

    #[must_use]
    pub fn unresolved(
        id: impl Into<String>,
        span: TextEditSpan,
        slot: UnresolvedPolicySlot,
    ) -> Self {
        Self {
            id: id.into(),
            span,
            content: PatchEditContent::UnresolvedSlot(slot),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchProposalDecision {
    Ready,
    Hold,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchProposalIssueKind {
    SchemaVersionMismatch,
    EmptyProposalId,
    EmptyPath,
    NoEdits,
    EmptyEditId,
    DuplicateEditId,
    EmptyPolicySlotId,
    UnknownEncoding,
    EncodingMismatch,
    MixedLineEndings,
    MissingLineEndingConvention,
    LineEndingMismatch,
    LineEndingTerminationMismatch,
    ReplacementLineEndingMismatch,
    SpanOutOfBounds,
    SpanNotUtf8Boundary,
    OverlappingSpans,
    UnresolvedPolicySlot,
    StaleBase,
    AnalysisBindingMismatch,
    StaleAnchorContext,
    OutputLengthOverflow,
}

impl PatchProposalIssueKind {
    const fn decision(self) -> PatchProposalDecision {
        match self {
            Self::MixedLineEndings
            | Self::MissingLineEndingConvention
            | Self::UnresolvedPolicySlot => PatchProposalDecision::Hold,
            Self::SchemaVersionMismatch
            | Self::EmptyProposalId
            | Self::EmptyPath
            | Self::NoEdits
            | Self::EmptyEditId
            | Self::DuplicateEditId
            | Self::EmptyPolicySlotId
            | Self::UnknownEncoding
            | Self::EncodingMismatch
            | Self::LineEndingMismatch
            | Self::LineEndingTerminationMismatch
            | Self::ReplacementLineEndingMismatch
            | Self::SpanOutOfBounds
            | Self::SpanNotUtf8Boundary
            | Self::OverlappingSpans
            | Self::StaleBase
            | Self::AnalysisBindingMismatch
            | Self::StaleAnchorContext
            | Self::OutputLengthOverflow => PatchProposalDecision::Reject,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposalIssue {
    pub kind: PatchProposalIssueKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_edit_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposalPreflight {
    pub decision: PatchProposalDecision,
    pub issues: Vec<PatchProposalIssue>,
}

impl PatchProposalPreflight {
    fn from_issues(issues: Vec<PatchProposalIssue>) -> Self {
        let decision = if issues
            .iter()
            .any(|issue| issue.kind.decision() == PatchProposalDecision::Reject)
        {
            PatchProposalDecision::Reject
        } else if issues
            .iter()
            .any(|issue| issue.kind.decision() == PatchProposalDecision::Hold)
        {
            PatchProposalDecision::Hold
        } else {
            PatchProposalDecision::Ready
        };
        Self { decision, issues }
    }

    #[must_use]
    pub fn has_issue(&self, kind: PatchProposalIssueKind) -> bool {
        self.issues.iter().any(|issue| issue.kind == kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposal {
    pub schema_version: String,
    pub id: String,
    pub path: String,
    pub base: TextDocumentBase,
    pub edits: Vec<PatchTextEdit>,
}

impl PatchProposal {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        base: TextDocumentBase,
        edits: Vec<PatchTextEdit>,
    ) -> Self {
        Self {
            schema_version: PATCH_PROPOSAL_SCHEMA_VERSION.to_string(),
            id: id.into(),
            path: path.into(),
            base,
            edits,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchProposalApplyError {
    Held(PatchProposalPreflight),
    Rejected(PatchProposalPreflight),
}

impl PatchProposalApplyError {
    #[must_use]
    pub const fn preflight(&self) -> &PatchProposalPreflight {
        match self {
            Self::Held(preflight) | Self::Rejected(preflight) => preflight,
        }
    }
}

impl Display for PatchProposalApplyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let (decision, preflight) = match self {
            Self::Held(preflight) => ("held", preflight),
            Self::Rejected(preflight) => ("rejected", preflight),
        };
        write!(
            formatter,
            "patch proposal {decision} by preflight with {} issue(s)",
            preflight.issues.len()
        )
    }
}

impl std::error::Error for PatchProposalApplyError {}
