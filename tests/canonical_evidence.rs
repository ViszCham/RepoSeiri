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

    assert_eq!(analysis.schema_version, "seiri.analysis.v1");
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
