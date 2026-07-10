use super::{
    detect_line_ending, PatchEditContent, PatchProposal, PatchProposalApplyError,
    PatchProposalDecision, PatchProposalIssue, PatchProposalIssueKind, PatchProposalPreflight,
    PatchTextEdit, TextDocumentBase, TextEncoding, TextLineEnding, PATCH_PROPOSAL_SCHEMA_VERSION,
};
use std::collections::BTreeSet;

impl PatchProposal {
    #[must_use]
    pub fn preflight_structure(&self) -> PatchProposalPreflight {
        let mut issues = Vec::new();
        if self.schema_version != PATCH_PROPOSAL_SCHEMA_VERSION {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::SchemaVersionMismatch,
                None,
                None,
            );
        }
        if self.id.is_empty() {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::EmptyProposalId,
                None,
                None,
            );
        }
        if self.path.is_empty() {
            push_issue(&mut issues, PatchProposalIssueKind::EmptyPath, None, None);
        }
        if self.edits.is_empty() {
            push_issue(&mut issues, PatchProposalIssueKind::NoEdits, None, None);
        }
        if self.base.encoding == TextEncoding::Unknown {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::UnknownEncoding,
                None,
                None,
            );
        }
        if self.base.line_ending == TextLineEnding::Mixed {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::MixedLineEndings,
                None,
                None,
            );
        }

        let mut edit_ids = BTreeSet::new();
        for edit in &self.edits {
            if edit.id.is_empty() {
                push_issue(&mut issues, PatchProposalIssueKind::EmptyEditId, None, None);
            } else if !edit_ids.insert(edit.id.as_str()) {
                push_issue(
                    &mut issues,
                    PatchProposalIssueKind::DuplicateEditId,
                    Some(edit.id.clone()),
                    None,
                );
            }
            if edit.span.byte_end > self.base.byte_len {
                push_issue(
                    &mut issues,
                    PatchProposalIssueKind::SpanOutOfBounds,
                    Some(edit.id.clone()),
                    None,
                );
            }
            match &edit.content {
                PatchEditContent::Literal(replacement) => validate_replacement_line_endings(
                    replacement,
                    self.base.line_ending,
                    &edit.id,
                    &mut issues,
                ),
                PatchEditContent::UnresolvedSlot(slot) => {
                    if slot.id.is_empty() {
                        push_issue(
                            &mut issues,
                            PatchProposalIssueKind::EmptyPolicySlotId,
                            Some(edit.id.clone()),
                            None,
                        );
                    }
                    push_issue(
                        &mut issues,
                        PatchProposalIssueKind::UnresolvedPolicySlot,
                        Some(edit.id.clone()),
                        None,
                    );
                }
            }
        }

        validate_edit_overlap(&self.edits, &mut issues);
        if expected_output_len(self.base.byte_len, &self.edits).is_none() {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::OutputLengthOverflow,
                None,
                None,
            );
        }
        PatchProposalPreflight::from_issues(issues)
    }

    #[must_use]
    pub fn preflight_against(&self, current: &[u8]) -> PatchProposalPreflight {
        let mut issues = self.preflight_structure().issues;
        let current_base = TextDocumentBase::from_bytes(current);

        if current_base.encoding == TextEncoding::Unknown {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::UnknownEncoding,
                None,
                None,
            );
        }
        if current_base.encoding != self.base.encoding {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::EncodingMismatch,
                None,
                None,
            );
        }
        if current_base.line_ending != self.base.line_ending {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::LineEndingMismatch,
                None,
                None,
            );
        }
        if current_base.ends_with_line_ending != self.base.ends_with_line_ending {
            push_issue(
                &mut issues,
                PatchProposalIssueKind::LineEndingTerminationMismatch,
                None,
                None,
            );
        }
        if current_base.digest != self.base.digest || current_base.byte_len != self.base.byte_len {
            push_issue(&mut issues, PatchProposalIssueKind::StaleBase, None, None);
        }

        if let Ok(text) = std::str::from_utf8(current) {
            for edit in &self.edits {
                if edit.span.byte_end <= current.len()
                    && (!text.is_char_boundary(edit.span.byte_start)
                        || !text.is_char_boundary(edit.span.byte_end))
                {
                    push_issue(
                        &mut issues,
                        PatchProposalIssueKind::SpanNotUtf8Boundary,
                        Some(edit.id.clone()),
                        None,
                    );
                }
            }
        }

        PatchProposalPreflight::from_issues(issues)
    }

    pub fn apply_to_bytes(&self, current: &[u8]) -> Result<Vec<u8>, PatchProposalApplyError> {
        let preflight = self.preflight_against(current);
        match preflight.decision {
            PatchProposalDecision::Hold => {
                return Err(PatchProposalApplyError::Held(preflight));
            }
            PatchProposalDecision::Reject => {
                return Err(PatchProposalApplyError::Rejected(preflight));
            }
            PatchProposalDecision::Ready => {}
        }

        let output_len = expected_output_len(current.len(), &self.edits)
            .expect("ready preflight guarantees representable output length");
        let mut ordered = self.edits.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|edit| (edit.span.byte_start, edit.span.byte_end, edit.id.as_str()));

        let mut output = Vec::with_capacity(output_len);
        let mut cursor = 0;
        for edit in ordered {
            output.extend_from_slice(&current[cursor..edit.span.byte_start]);
            let PatchEditContent::Literal(replacement) = &edit.content else {
                unreachable!("ready preflight contains no unresolved policy slots");
            };
            output.extend_from_slice(replacement.as_bytes());
            cursor = edit.span.byte_end;
        }
        output.extend_from_slice(&current[cursor..]);
        Ok(output)
    }
}

fn validate_replacement_line_endings(
    replacement: &str,
    base_line_ending: TextLineEnding,
    edit_id: &str,
    issues: &mut Vec<PatchProposalIssue>,
) {
    let replacement_line_ending = detect_line_ending(replacement.as_bytes());
    if replacement_line_ending == TextLineEnding::None {
        return;
    }
    match base_line_ending {
        TextLineEnding::None => push_issue(
            issues,
            PatchProposalIssueKind::MissingLineEndingConvention,
            Some(edit_id.to_string()),
            None,
        ),
        TextLineEnding::Mixed => {}
        expected if expected != replacement_line_ending => push_issue(
            issues,
            PatchProposalIssueKind::ReplacementLineEndingMismatch,
            Some(edit_id.to_string()),
            None,
        ),
        TextLineEnding::Lf | TextLineEnding::CrLf => {}
    }
}

fn validate_edit_overlap(edits: &[PatchTextEdit], issues: &mut Vec<PatchProposalIssue>) {
    let mut ordered = edits.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|edit| (edit.span.byte_start, edit.span.byte_end, edit.id.as_str()));
    for pair in ordered.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let overlaps = right.span.byte_start < left.span.byte_end
            || (left.span.byte_start == left.span.byte_end
                && left.span.byte_start == right.span.byte_start);
        if overlaps {
            push_issue(
                issues,
                PatchProposalIssueKind::OverlappingSpans,
                Some(left.id.clone()),
                Some(right.id.clone()),
            );
        }
    }
}

fn expected_output_len(base_len: usize, edits: &[PatchTextEdit]) -> Option<usize> {
    edits.iter().try_fold(base_len, |length, edit| {
        let replacement_len = match &edit.content {
            PatchEditContent::Literal(replacement) => replacement.len(),
            PatchEditContent::UnresolvedSlot(_) => 0,
        };
        length
            .checked_sub(edit.span.replaced_bytes())?
            .checked_add(replacement_len)
    })
}

fn push_issue(
    issues: &mut Vec<PatchProposalIssue>,
    kind: PatchProposalIssueKind,
    edit_id: Option<String>,
    related_edit_id: Option<String>,
) {
    let issue = PatchProposalIssue {
        kind,
        edit_id,
        related_edit_id,
    };
    if !issues.contains(&issue) {
        issues.push(issue);
    }
}
