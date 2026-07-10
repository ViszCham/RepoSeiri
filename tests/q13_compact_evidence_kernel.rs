use seiri_core::{
    stable_evidence_id, EvidenceConfidence, EvidenceDraft, EvidenceEvent, EvidenceId,
    EvidenceKernel, EvidenceKind, EvidenceOrigin, EvidenceScanner, EvidenceScope, RouteKind,
    SourceSpan,
};
use std::collections::BTreeSet;
use std::mem::size_of;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn q13_evidence_id_is_compact_deterministic_and_strict() {
    assert_eq!(size_of::<EvidenceId>(), size_of::<u32>());

    let id = stable_evidence_id(7);
    assert_eq!(id.ordinal(), 7);
    assert_eq!(id.to_string(), "evrec-0007");
    assert_eq!(
        serde_json::to_string(&id).expect("serialize evidence id"),
        "\"evrec-0007\""
    );
    assert_eq!(
        serde_json::from_str::<EvidenceId>("\"evrec-0007\"").expect("deserialize evidence id"),
        id
    );

    for invalid in ["evrec-0000", "evrec-7", "evidence-0007", "evrec-4294967296"] {
        assert!(
            invalid.parse::<EvidenceId>().is_err(),
            "accepted invalid evidence id {invalid}"
        );
    }
}

#[test]
fn q13_source_span_rejects_invalid_wire_states() {
    let span = SourceSpan::new(3, 5, 20, 38);
    assert!(span.is_valid());
    let roundtrip = serde_json::from_value::<SourceSpan>(
        serde_json::to_value(span).expect("serialize source span"),
    )
    .expect("deserialize source span");
    assert_eq!(roundtrip, span);

    for invalid in [
        serde_json::json!({"line": 0, "column": 1, "byte_start": 0, "byte_end": 1}),
        serde_json::json!({"line": 1, "column": 0, "byte_start": 0, "byte_end": 1}),
        serde_json::json!({"line": 1, "column": 1, "byte_start": 4, "byte_end": 3}),
    ] {
        assert!(serde_json::from_value::<SourceSpan>(invalid).is_err());
    }
}

#[test]
fn q13_kernel_assigns_ids_and_rejects_non_contiguous_wire_order() {
    let kernel = EvidenceKernel::from_drafts(vec![
        draft(EvidenceKind::ReadmePresent, None),
        draft(
            EvidenceKind::MarkdownHeading,
            Some(SourceSpan::new(2, 1, 10, 20)),
        ),
    ])
    .expect("valid evidence drafts");

    assert_eq!(kernel.len(), 2);
    assert_eq!(kernel.facts()[0].id, stable_evidence_id(1));
    assert_eq!(kernel.facts()[1].id, stable_evidence_id(2));
    assert_eq!(kernel.facts()[1].span, Some(SourceSpan::new(2, 1, 10, 20)));

    let mut wire = serde_json::to_value(&kernel).expect("serialize kernel");
    wire["facts"][1]["id"] = serde_json::json!("evrec-0003");
    assert!(serde_json::from_value::<EvidenceKernel>(wire).is_err());
}

#[test]
fn q13_kernel_rejects_origin_mismatch_and_missing_markdown_span() {
    let mut mismatched = draft(EvidenceKind::ReadmePresent, None);
    mismatched.origin.event = EvidenceEvent::MarkdownHeading;
    assert!(EvidenceKernel::from_drafts(vec![mismatched]).is_err());

    let missing_span = draft(EvidenceKind::MarkdownHeading, None);
    assert!(EvidenceKernel::from_drafts(vec![missing_span]).is_err());
}

#[test]
fn q13_audit_uses_canonical_facts_and_projects_legacy_views() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");
    let repeated =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("repeat audit");

    assert!(!snapshot.evidence_kernel.is_empty());
    assert_eq!(snapshot.evidence_kernel, repeated.evidence_kernel);
    assert_eq!(snapshot.evidence_kernel.len(), snapshot.evidence.len());
    assert_eq!(
        snapshot.evidence_kernel.len(),
        snapshot.evidence_ledger.len()
    );

    for ((fact, legacy), ledger) in snapshot
        .evidence_kernel
        .facts()
        .iter()
        .zip(&snapshot.evidence)
        .zip(&snapshot.evidence_ledger)
    {
        assert_eq!(ledger.id, fact.id);
        assert_eq!(
            ledger.legacy_evidence_id.as_deref(),
            Some(legacy.id.as_str())
        );
        assert_eq!(ledger.kind, fact.kind);
        assert_eq!(ledger.route, fact.route);
        assert_eq!(ledger.scope, fact.scope);
        assert_eq!(ledger.confidence, fact.confidence);
        assert_eq!(
            ledger.span.map(|span| span.start_line),
            fact.span.map(|span| span.line)
        );
    }

    let ids = snapshot
        .evidence_kernel
        .facts()
        .iter()
        .map(|fact| fact.id)
        .collect::<BTreeSet<_>>();
    assert!(snapshot
        .pattern_matches
        .iter()
        .flat_map(|pattern| &pattern.evidence_ids)
        .all(|id| ids.contains(id)));
    assert!(snapshot
        .route_states
        .iter()
        .flat_map(|state| &state.evidence_ids)
        .all(|id| ids.contains(id)));

    let readme = snapshot.readme.as_ref().expect("README summary");
    let docs_link = readme
        .links
        .iter()
        .find(|link| link.target == "docs/quickstart.md")
        .expect("docs link");
    let fact = snapshot
        .evidence_kernel
        .facts()
        .iter()
        .find(|fact| fact.kind == EvidenceKind::MarkdownLink && fact.route == Some(RouteKind::Docs))
        .expect("docs link fact");
    assert_eq!(fact.span, docs_link.span);

    let mut kernel_only = snapshot.clone();
    kernel_only.evidence.clear();
    kernel_only.evidence_ledger.clear();
    assert_eq!(
        seiri_patterns::common_registry().evaluate_patterns(&kernel_only),
        snapshot.pattern_matches
    );

    let json = seiri_report::to_json(&snapshot).expect("snapshot JSON");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse snapshot JSON");
    assert!(parsed["evidence_kernel"]["facts"].is_array());
    assert!(parsed["evidence"].is_array());
    assert!(parsed["evidence_ledger"].is_array());
}

fn draft(kind: EvidenceKind, span: Option<SourceSpan>) -> EvidenceDraft {
    EvidenceDraft {
        kind,
        path: Some("README.md".to_string()),
        route: Some(RouteKind::Identity),
        value: "fixture evidence".to_string(),
        origin: EvidenceOrigin {
            scanner: EvidenceScanner::Markdown,
            event: if kind == EvidenceKind::MarkdownHeading {
                EvidenceEvent::MarkdownHeading
            } else {
                EvidenceEvent::ReadmeDiscovery
            },
        },
        scope: EvidenceScope::Root,
        confidence: EvidenceConfidence::High,
        span,
    }
}
