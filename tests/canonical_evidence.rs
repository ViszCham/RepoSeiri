use seiri_core::{
    EvidenceAtom, EvidenceConfidence, EvidenceDraft, EvidenceKernel, EvidenceKernelError,
    EvidenceProducer, MarkdownEvidenceKind, ProfileKind, ReadmePresence, SourceDomain, SourceSpan,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn audit_builds_typed_evidence_directly_with_canonical_ids() {
    let fixture = repository("canonical-evidence");
    let analysis =
        seiri_report::audit_repository_with_profile(&fixture, ProfileKind::Library).expect("audit");

    assert_eq!(analysis.schema_version, "seiri.analysis.v2");
    assert!(!analysis.evidence_kernel.is_empty());
    for (index, fact) in analysis.evidence_kernel.facts().iter().enumerate() {
        assert_eq!(fact.id.ordinal(), (index + 1) as u32);
    }
    assert!(analysis.evidence_kernel.facts().iter().any(|fact| {
        fact.atom == EvidenceAtom::Readme(ReadmePresence::Present)
            && analysis.evidence_kernel.path_for_fact(fact) == Some("README.md")
    }));
    assert!(analysis.evidence_kernel.facts().iter().any(|fact| matches!(
        fact.atom,
        EvidenceAtom::Markdown {
            event: MarkdownEvidenceKind::Link,
            ..
        }
    )));

    let json = seiri_report::to_json(&analysis).expect("audit JSON");
    assert!(json.contains("\"evidence_kernel\""));
    for removed in ["evidence_ledger", "route_states", "evidence_kernel_v2"] {
        assert!(!json.contains(removed), "removed key leaked: {removed}");
    }
    fs::remove_dir_all(fixture).expect("remove temp repository");
}

#[test]
fn evidence_kernel_rejects_invalid_typed_producer_and_span_shapes() {
    let invalid_producer = EvidenceDraft {
        atom: EvidenceAtom::FilePresent,
        domain: SourceDomain::RepositoryLocal,
        producer: EvidenceProducer::Markdown,
        path: Some("README.md".to_string()),
        span: None,
        confidence: EvidenceConfidence::High,
    };
    assert_eq!(
        EvidenceKernel::from_drafts(vec![invalid_producer]),
        Err(EvidenceKernelError::ProducerAtomMismatch)
    );

    let missing_span = EvidenceDraft {
        atom: EvidenceAtom::Markdown {
            event: MarkdownEvidenceKind::Heading,
            route: None,
        },
        domain: SourceDomain::RepositoryLocal,
        producer: EvidenceProducer::Markdown,
        path: Some("README.md".to_string()),
        span: None,
        confidence: EvidenceConfidence::Medium,
    };
    assert_eq!(
        EvidenceKernel::from_drafts(vec![missing_span]),
        Err(EvidenceKernelError::MissingSourceSpan)
    );

    let valid = EvidenceDraft {
        span: Some(SourceSpan {
            line: 1,
            column: 1,
            byte_start: 0,
            byte_end: 3,
        }),
        ..EvidenceDraft {
            atom: EvidenceAtom::Markdown {
                event: MarkdownEvidenceKind::Heading,
                route: None,
            },
            domain: SourceDomain::RepositoryLocal,
            producer: EvidenceProducer::Markdown,
            path: Some("README.md".to_string()),
            span: None,
            confidence: EvidenceConfidence::Medium,
        }
    };
    assert_eq!(
        EvidenceKernel::from_drafts(vec![valid])
            .expect("kernel")
            .len(),
        1
    );
}

#[test]
fn evidence_fingerprint_is_independent_of_storage_ordinal() {
    let first_root = repository("fingerprint-first");
    let first = seiri_report::audit_repository(&first_root).expect("first analysis");
    let fact = first
        .evidence_kernel
        .facts()
        .iter()
        .find(|fact| first.evidence_kernel.path_for_fact(fact) == Some("LICENSE"))
        .expect("LICENSE evidence");
    let mut shifted = fact.clone();
    shifted.id = seiri_core::EvidenceId::from_ordinal(999).expect("alternate ordinal");
    assert_ne!(fact.id, shifted.id);
    assert_eq!(
        seiri_delta::evidence_fingerprint(&first, fact).expect("original fingerprint"),
        seiri_delta::evidence_fingerprint(&first, &shifted).expect("shifted fingerprint")
    );
    fs::remove_dir_all(first_root).expect("cleanup first");
}

#[test]
fn evidence_identity_binds_normalized_markdown_target() {
    let first_root = repository("target-first");
    let second_root = repository("target-second");
    fs::write(
        second_root.join("README.md"),
        "# Example\n\n[Documentation](docs/other.md)\n",
    )
    .expect("alternate README");
    fs::write(second_root.join("docs/other.md"), "# Other\n").expect("alternate target");
    let first = seiri_report::audit_repository(&first_root).expect("first analysis");
    let second = seiri_report::audit_repository(&second_root).expect("second analysis");
    let link_fingerprint = |analysis: &seiri_core::RepositoryAnalysis| {
        let fact = analysis
            .evidence_kernel
            .facts()
            .iter()
            .find(|fact| {
                analysis.evidence_kernel.path_for_fact(fact) == Some("README.md")
                    && matches!(
                        fact.atom,
                        EvidenceAtom::Markdown {
                            event: seiri_core::MarkdownEvidenceKind::Link,
                            ..
                        }
                    )
            })
            .expect("link evidence");
        seiri_delta::evidence_fingerprint(analysis, fact).expect("fingerprint")
    };
    assert_ne!(
        link_fingerprint(&first).identity,
        link_fingerprint(&second).identity
    );
    fs::remove_dir_all(first_root).expect("cleanup first");
    fs::remove_dir_all(second_root).expect("cleanup second");
}

fn repository(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-{name}-{nonce}"));
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(
        root.join("README.md"),
        "# Example\n\n[Documentation](docs/README.md)\n",
    )
    .expect("README");
    fs::write(root.join("docs/README.md"), "# Documentation\n").expect("docs");
    fs::write(root.join("LICENSE"), "MIT\n").expect("license");
    root
}
