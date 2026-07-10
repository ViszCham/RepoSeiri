use super::{
    PatchBaseDigest, PatchProposal, PatchProposalApplyError, PatchProposalDecision,
    PatchProposalIssue, PatchProposalIssueKind, PatchProposalPreflight, TextDocumentBase,
    TextEditSpan,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub const PATCH_ANCHOR_CONTEXT_BYTES: usize = 96;

/// Identifies the bounded analysis inputs from which a patch plan was derived.
///
/// The digest is deterministic metadata, not a cryptographic integrity primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchAnalysisRun {
    pub id: String,
    pub snapshot_digest: PatchBaseDigest,
}

impl PatchAnalysisRun {
    #[must_use]
    pub fn new(id: impl Into<String>, snapshot_digest: PatchBaseDigest) -> Self {
        Self {
            id: id.into(),
            snapshot_digest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchAnchorSlice {
    pub byte_len: usize,
    pub digest: PatchBaseDigest,
}

impl PatchAnchorSlice {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            byte_len: bytes.len(),
            digest: PatchBaseDigest::from_bytes(bytes),
        }
    }

    fn matches(self, bytes: &[u8]) -> bool {
        self.byte_len == bytes.len() && self.digest == PatchBaseDigest::from_bytes(bytes)
    }
}

/// Captures the local bytes around one edit without retaining the source text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchAnchorContext {
    pub edit_id: String,
    pub span: TextEditSpan,
    pub before: PatchAnchorSlice,
    pub target: PatchAnchorSlice,
    pub after: PatchAnchorSlice,
}

impl PatchAnchorContext {
    fn from_current(edit_id: String, span: TextEditSpan, current: &[u8]) -> Self {
        let before_start = span.byte_start.saturating_sub(PATCH_ANCHOR_CONTEXT_BYTES);
        let after_end = span
            .byte_end
            .saturating_add(PATCH_ANCHOR_CONTEXT_BYTES)
            .min(current.len());
        Self {
            edit_id,
            span,
            before: PatchAnchorSlice::from_bytes(&current[before_start..span.byte_start]),
            target: PatchAnchorSlice::from_bytes(&current[span.byte_start..span.byte_end]),
            after: PatchAnchorSlice::from_bytes(&current[span.byte_end..after_end]),
        }
    }

    fn matches_current(&self, current: &[u8]) -> bool {
        if self.span.byte_end > current.len() || self.span.byte_start > self.span.byte_end {
            return false;
        }
        let before_start = self
            .span
            .byte_start
            .saturating_sub(PATCH_ANCHOR_CONTEXT_BYTES);
        let after_end = self
            .span
            .byte_end
            .saturating_add(PATCH_ANCHOR_CONTEXT_BYTES)
            .min(current.len());
        self.before
            .matches(&current[before_start..self.span.byte_start])
            && self
                .target
                .matches(&current[self.span.byte_start..self.span.byte_end])
            && self.after.matches(&current[self.span.byte_end..after_end])
    }
}

/// Binds one proposal to its analysis run, document base, and edit anchors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposalBinding {
    pub analysis_run: PatchAnalysisRun,
    pub proposal_id: String,
    pub path: String,
    pub base_digest: PatchBaseDigest,
    pub anchors: Vec<PatchAnchorContext>,
}

impl PatchProposalBinding {
    pub fn bind(
        analysis_run: PatchAnalysisRun,
        proposal: &PatchProposal,
        current: &[u8],
    ) -> Result<Self, PatchProposalBindingError> {
        let current_base = TextDocumentBase::from_bytes(current);
        if current_base.digest() != proposal.base.digest()
            || current_base.byte_len() != proposal.base.byte_len()
        {
            return Err(PatchProposalBindingError::StaleBase);
        }

        let mut anchors = Vec::with_capacity(proposal.edits.len());
        for edit in &proposal.edits {
            if edit.span.byte_end > current.len() {
                return Err(PatchProposalBindingError::SpanOutOfBounds {
                    edit_id: edit.id.clone(),
                });
            }
            anchors.push(PatchAnchorContext::from_current(
                edit.id.clone(),
                edit.span,
                current,
            ));
        }

        Ok(Self {
            analysis_run,
            proposal_id: proposal.id.clone(),
            path: proposal.path.clone(),
            base_digest: proposal.base.digest(),
            anchors,
        })
    }

    #[must_use]
    pub fn preflight_against(
        &self,
        proposal: &PatchProposal,
        current: &[u8],
    ) -> PatchProposalPreflight {
        let mut issues = proposal.preflight_against(current).issues;
        if self.analysis_run.id.is_empty()
            || self.proposal_id != proposal.id
            || self.path != proposal.path
            || self.base_digest != proposal.base.digest()
            || self.anchors.len() != proposal.edits.len()
        {
            issues.push(PatchProposalIssue {
                kind: PatchProposalIssueKind::AnalysisBindingMismatch,
                edit_id: None,
                related_edit_id: None,
            });
        }

        for anchor in &self.anchors {
            let matching_edit = proposal
                .edits
                .iter()
                .find(|edit| edit.id == anchor.edit_id && edit.span == anchor.span);
            if matching_edit.is_none() || !anchor.matches_current(current) {
                issues.push(PatchProposalIssue {
                    kind: PatchProposalIssueKind::StaleAnchorContext,
                    edit_id: Some(anchor.edit_id.clone()),
                    related_edit_id: None,
                });
            }
        }

        PatchProposalPreflight::from_issues(issues)
    }

    pub fn apply_to_bytes(
        &self,
        proposal: &PatchProposal,
        current: &[u8],
    ) -> Result<Vec<u8>, PatchProposalApplyError> {
        let preflight = self.preflight_against(proposal, current);
        match preflight.decision {
            PatchProposalDecision::Ready => proposal.apply_to_bytes(current),
            PatchProposalDecision::Hold => Err(PatchProposalApplyError::Held(preflight)),
            PatchProposalDecision::Reject => Err(PatchProposalApplyError::Rejected(preflight)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchProposalBindingError {
    StaleBase,
    SpanOutOfBounds { edit_id: String },
}

impl Display for PatchProposalBindingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleBase => formatter.write_str("analysis source does not match proposal base"),
            Self::SpanOutOfBounds { edit_id } => {
                write!(
                    formatter,
                    "analysis anchor span is out of bounds for edit `{edit_id}`"
                )
            }
        }
    }
}

impl std::error::Error for PatchProposalBindingError {}
