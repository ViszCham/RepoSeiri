use seiri_core::{
    PatchProposal, PatchProposalApplyError, PatchProposalDecision, PatchProposalIssueKind,
    PatchTextEdit, PolicySlotKind, TextDocumentBase, TextEditSpan, TextEncoding, TextLineEnding,
    UnresolvedPolicySlot, PATCH_PROPOSAL_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn literal_proposal(source: &[u8], edits: Vec<PatchTextEdit>) -> PatchProposal {
    PatchProposal::new(
        "proposal-test",
        "README.md",
        TextDocumentBase::from_bytes(source),
        edits,
    )
}

#[test]
fn q18_planner_binds_safe_edit_to_scanner_base_and_applies_only_in_memory() {
    let root = fixture("safe-plan-repo");
    let source = fs::read(root.join("README.md")).expect("README bytes");
    let snapshot =
        seiri_report::audit_repository_with_profile(&root, seiri_core::ProfileKind::Common)
            .expect("audit fixture");
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    let operation = plan.operations.first().expect("safe docs route operation");
    let document = snapshot
        .readme_document
        .as_ref()
        .expect("scanner base metadata");

    assert_eq!(
        operation.proposal.schema_version,
        PATCH_PROPOSAL_SCHEMA_VERSION
    );
    assert_eq!(&operation.proposal.base, document.base());
    assert_eq!(operation.proposal.base.encoding(), TextEncoding::Utf8);
    assert_eq!(operation.proposal.base.line_ending(), TextLineEnding::Lf);
    assert_eq!(operation.proposal.base.byte_len(), source.len());
    assert_eq!(operation.proposal.edits.len(), 1);
    assert_eq!(
        operation.proposal.edits[0].span,
        TextEditSpan::insertion(source.len())
    );
    assert_eq!(
        operation.proposal.preflight_against(&source).decision,
        PatchProposalDecision::Ready
    );

    let output = operation
        .proposal
        .apply_to_bytes(&source)
        .expect("ready proposal applies to an owned byte buffer");
    assert_eq!(&output[..source.len()], source.as_slice());
    assert!(String::from_utf8(output)
        .expect("UTF-8 output")
        .contains("## Documentation\n\n- [Documentation](docs/)"));
    assert_eq!(
        fs::read(root.join("README.md")).expect("README remains unchanged"),
        source
    );
}

#[test]
fn q18_stale_base_is_rejected_before_application() {
    let source = b"# Demo\n";
    let proposal = literal_proposal(
        source,
        vec![PatchTextEdit::literal(
            "append",
            TextEditSpan::insertion(source.len()),
            "\nMore\n",
        )],
    );
    let stale = b"# Damo\n";

    let preflight = proposal.preflight_against(stale);
    assert_eq!(preflight.decision, PatchProposalDecision::Reject);
    assert!(preflight.has_issue(PatchProposalIssueKind::StaleBase));
    let error = proposal
        .apply_to_bytes(stale)
        .expect_err("stale base must not be applied");
    assert!(matches!(error, PatchProposalApplyError::Rejected(_)));
    assert!(error
        .preflight()
        .has_issue(PatchProposalIssueKind::StaleBase));
}

#[test]
fn q18_overlap_and_coincident_insertions_are_rejected() {
    let source = b"abcdef\n";
    let overlap = literal_proposal(
        source,
        vec![
            PatchTextEdit::literal("left", TextEditSpan::new(1, 4).expect("ordered span"), "L"),
            PatchTextEdit::literal("right", TextEditSpan::new(3, 5).expect("ordered span"), "R"),
        ],
    );
    let coincident = literal_proposal(
        source,
        vec![
            PatchTextEdit::literal("first", TextEditSpan::insertion(2), "A"),
            PatchTextEdit::literal("second", TextEditSpan::insertion(2), "B"),
        ],
    );

    for proposal in [overlap, coincident] {
        let preflight = proposal.preflight_against(source);
        assert_eq!(preflight.decision, PatchProposalDecision::Reject);
        assert!(preflight.has_issue(PatchProposalIssueKind::OverlappingSpans));
        assert!(proposal.apply_to_bytes(source).is_err());
    }
}

#[test]
fn q18_unknown_encoding_and_utf8_split_spans_are_rejected() {
    let unknown = [0xff, 0xfe];
    let unknown_proposal = literal_proposal(
        &unknown,
        vec![PatchTextEdit::literal(
            "append",
            TextEditSpan::insertion(unknown.len()),
            "x",
        )],
    );
    assert_eq!(unknown_proposal.base.encoding(), TextEncoding::Unknown);
    assert!(unknown_proposal
        .preflight_against(&unknown)
        .has_issue(PatchProposalIssueKind::UnknownEncoding));
    assert!(unknown_proposal.apply_to_bytes(&unknown).is_err());

    let utf8 = "é\n".as_bytes();
    let split = literal_proposal(
        utf8,
        vec![PatchTextEdit::literal(
            "split-codepoint",
            TextEditSpan::insertion(1),
            "x",
        )],
    );
    assert_eq!(
        split.preflight_structure().decision,
        PatchProposalDecision::Ready
    );
    let preflight = split.preflight_against(utf8);
    assert_eq!(preflight.decision, PatchProposalDecision::Reject);
    assert!(preflight.has_issue(PatchProposalIssueKind::SpanNotUtf8Boundary));
}

#[test]
fn q18_unresolved_policy_content_is_held() {
    let source = b"# Security\n";
    let proposal = PatchProposal::new(
        "security-policy-draft",
        "SECURITY.md",
        TextDocumentBase::from_bytes(source),
        vec![PatchTextEdit::unresolved(
            "policy-body",
            TextEditSpan::insertion(source.len()),
            UnresolvedPolicySlot::new(
                "disclosure-channel",
                PolicySlotKind::SecurityPolicy,
                "Maintainer must choose a private vulnerability reporting channel.",
            ),
        )],
    );

    let preflight = proposal.preflight_against(source);
    assert_eq!(preflight.decision, PatchProposalDecision::Hold);
    assert!(preflight.has_issue(PatchProposalIssueKind::UnresolvedPolicySlot));
    assert!(matches!(
        proposal.apply_to_bytes(source),
        Err(PatchProposalApplyError::Held(_))
    ));
}

#[test]
fn q18_replacement_eol_and_current_eol_mismatches_are_rejected() {
    let source = b"# Demo\r\n";
    let replacement_mismatch = literal_proposal(
        source,
        vec![PatchTextEdit::literal(
            "append",
            TextEditSpan::insertion(source.len()),
            "\nMore\n",
        )],
    );
    assert_eq!(
        replacement_mismatch.base.line_ending(),
        TextLineEnding::CrLf
    );
    let structural = replacement_mismatch.preflight_structure();
    assert_eq!(structural.decision, PatchProposalDecision::Reject);
    assert!(structural.has_issue(PatchProposalIssueKind::ReplacementLineEndingMismatch));

    let valid = literal_proposal(
        source,
        vec![PatchTextEdit::literal(
            "append",
            TextEditSpan::insertion(source.len()),
            "\r\nMore\r\n",
        )],
    );
    let current_lf = b"# Demo\n";
    let current_preflight = valid.preflight_against(current_lf);
    assert_eq!(current_preflight.decision, PatchProposalDecision::Reject);
    assert!(current_preflight.has_issue(PatchProposalIssueKind::LineEndingMismatch));
    assert!(current_preflight.has_issue(PatchProposalIssueKind::StaleBase));
}

#[test]
fn q18_wire_rejects_inverted_spans_and_malformed_digests() {
    let source = b"# Demo\n";
    let proposal = literal_proposal(
        source,
        vec![PatchTextEdit::literal(
            "append",
            TextEditSpan::insertion(source.len()),
            "More",
        )],
    );

    let mut inverted = serde_json::to_value(&proposal).expect("proposal JSON");
    inverted["edits"][0]["span"]["byte_start"] = serde_json::json!(2);
    inverted["edits"][0]["span"]["byte_end"] = serde_json::json!(1);
    assert!(serde_json::from_value::<PatchProposal>(inverted).is_err());

    let mut malformed_digest = serde_json::to_value(&proposal).expect("proposal JSON");
    malformed_digest["base"]["digest"] = serde_json::json!("fnv1a64:ABCDEF0123456789");
    assert!(serde_json::from_value::<PatchProposal>(malformed_digest).is_err());

    let mut wrong_schema = proposal.clone();
    wrong_schema.schema_version = "seiri.patch_proposal.v0".to_string();
    assert!(wrong_schema
        .preflight_against(source)
        .has_issue(PatchProposalIssueKind::SchemaVersionMismatch));

    let mut wrong_termination = serde_json::to_value(&proposal).expect("proposal JSON");
    wrong_termination["base"]["ends_with_line_ending"] = serde_json::json!(false);
    let wrong_termination =
        serde_json::from_value::<PatchProposal>(wrong_termination).expect("typed proposal");
    assert!(wrong_termination
        .preflight_against(source)
        .has_issue(PatchProposalIssueKind::LineEndingTerminationMismatch));
}
