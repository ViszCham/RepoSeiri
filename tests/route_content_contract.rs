use seiri_core::{
    route_content_contract, ClaimBoundaryKind, DocumentDiagnosticKind, DocumentEvent,
    MarkdownLinkKind, Observation, ReviewGap, RouteKind,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn markdown_link_forms_keep_exact_source_spans() {
    let source = "[inline](docs/a.md)\n[reference][docs]\n[docs]\n<https://example.com>\n![image](img/a.png)\n![reference image][logo]\n<a class=\"x\" href=\"docs/b.md\">HTML</a>\n\n[docs]: docs/ref.md\n[logo]: img/ref.png\n";
    let scan = seiri_markdown::scan_document("README.md", source).expect("scan");
    let links = scan
        .events()
        .iter()
        .filter_map(|event| match event {
            DocumentEvent::Link(link) => Some(link),
            _ => None,
        })
        .collect::<Vec<_>>();

    for kind in [
        MarkdownLinkKind::Inline,
        MarkdownLinkKind::Reference,
        MarkdownLinkKind::Autolink,
        MarkdownLinkKind::Image,
        MarkdownLinkKind::HtmlAnchor,
    ] {
        let link = links
            .iter()
            .find(|link| link.kind == kind)
            .expect("link kind");
        let span = link.span.expect("byte-spanned event");
        let source_slice = &source[span.byte_start..span.byte_end];
        assert!(!source_slice.is_empty());
        assert!(source_slice.contains(&link.text) || kind == MarkdownLinkKind::Autolink);
    }
    assert!(links.iter().any(|link| {
        link.kind == MarkdownLinkKind::Reference
            && link.text == "docs"
            && link.target == "docs/ref.md"
    }));
    assert!(links.iter().any(|link| {
        link.kind == MarkdownLinkKind::Image
            && link.text == "reference image"
            && link.target == "img/ref.png"
    }));
}

#[test]
fn bounded_html_reports_unsupported_and_attribute_limits() {
    let attributes = (0..33)
        .map(|index| format!(" a{index}=\"x\""))
        .collect::<String>();
    let source = format!(
        "<a{attributes} href=\"docs/a.md\">Docs</a>\n<a href=\"x\">nested <b>text</b></a>\n"
    );
    let scan = seiri_markdown::scan_document("README.md", &source).expect("bounded scan");
    assert!(scan.diagnostics().iter().any(|diagnostic| {
        diagnostic.kind == DocumentDiagnosticKind::HtmlAttributeLimitExceeded
    }));
    assert!(scan
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.kind == DocumentDiagnosticKind::UnsupportedHtml));
}

#[test]
fn registry_covers_all_routes_and_claim_boundaries() {
    let registry = route_content_contract();
    assert_eq!(registry.len(), 63);
    for route in [
        RouteKind::Identity,
        RouteKind::Docs,
        RouteKind::Quickstart,
        RouteKind::Support,
        RouteKind::Intake,
        RouteKind::Contributing,
        RouteKind::Security,
        RouteKind::Release,
        RouteKind::Lifecycle,
        RouteKind::Governance,
        RouteKind::License,
        RouteKind::Automation,
        RouteKind::Ownership,
        RouteKind::Hygiene,
    ] {
        assert!(registry.iter().any(|slot| slot.route == route), "{route:?}");
    }
    let security = registry
        .iter()
        .find(|slot| slot.code == "security.private_disclosure")
        .expect("security slot");
    assert!(security
        .does_not_indicate
        .contains(&ClaimBoundaryKind::NotSecurityGuarantee));
    assert!(security
        .document_roles
        .contains(seiri_core::DocumentRole::SecurityPolicy));
    assert!(!security
        .document_roles
        .contains(seiri_core::DocumentRole::ReleaseNotes));
    let expected = registry
        .iter()
        .find(|slot| slot.code == "quickstart.expected_output")
        .expect("expected output slot");
    assert!(expected
        .does_not_indicate
        .contains(&ClaimBoundaryKind::NotRuntimeVerification));
}

#[test]
fn audit_builds_structural_pair_and_separate_content_priorities() {
    let root = temporary_repository("structural-pair");
    fs::write(
        root.join("README.md"),
        "# RepoSeiri\n\n## 日本語\n[ガイド](docs/guide.md)\n\n## English\n[Guide](./docs/guide.md)\n\n## Quickstart\nInstall with cargo.\n",
    )
    .expect("README");
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(root.join("docs/guide.md"), "# Guide\n").expect("guide");

    let snapshot = seiri_report::audit_repository(&root).expect("audit");
    assert_eq!(snapshot.route_content.assessments.len(), 63);
    assert_eq!(snapshot.route_content.structural_pairs.len(), 1);
    assert!(snapshot.route_content.structural_pairs[0].candidate_only);
    assert!(snapshot.document_consistency.conflicts.is_empty());
    assert!(snapshot
        .review_priority
        .priorities
        .iter()
        .any(|priority| { matches!(priority.gap, ReviewGap::ContentSlot { .. }) }));
    assert!(snapshot
        .route_content
        .assessments
        .iter()
        .filter(|item| item.enabled)
        .all(|item| !matches!(item.observation, Observation::Unknown(_))));

    let json = seiri_report::to_json(&snapshot).expect("json");
    assert!(json.contains("route_content"));
    let markdown = seiri_report::to_markdown(&snapshot);
    assert!(markdown.contains("Route Content Contract"));
    assert!(markdown.contains("does not indicate `NotSecurityGuarantee`"));

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn invalid_utf8_and_event_budget_become_unknown_not_absent() {
    let invalid_root = temporary_repository("invalid-utf8");
    fs::write(invalid_root.join("README.md"), [0xff, 0xfe]).expect("invalid README");
    let invalid = seiri_report::audit_repository(&invalid_root).expect("invalid audit");
    assert!(invalid.route_content.assessments.iter().any(|item| {
        matches!(
            item.observation,
            Observation::Unknown(seiri_core::UnknownReason::InvalidUtf8)
        )
    }));

    let limited_root = temporary_repository("event-limit");
    fs::write(limited_root.join("README.md"), "# Docs\n[Guide](docs.md)\n")
        .expect("limited README");
    let options = seiri_markdown::DocumentIndexOptions {
        document: seiri_markdown::DocumentScanOptions {
            max_events: 1,
            ..seiri_markdown::DocumentScanOptions::default()
        },
        ..seiri_markdown::DocumentIndexOptions::default()
    };
    let limited = seiri_report::audit_repository_with_options(
        &limited_root,
        seiri_core::ProfileKind::Common,
        &seiri_fs::ScanOptions::default(),
        &options,
    )
    .expect("limited audit");
    assert!(limited.route_content.assessments.iter().any(|item| {
        matches!(
            item.observation,
            Observation::Unknown(seiri_core::UnknownReason::LimitExceeded)
        )
    }));

    fs::remove_dir_all(invalid_root).expect("invalid cleanup");
    fs::remove_dir_all(limited_root).expect("limited cleanup");
}

#[test]
fn license_readme_route_requires_a_bounded_root_readme_marker() {
    let root = temporary_repository("license-marker-boundary");
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(
        root.join("README.md"),
        "# Demo\n\nThis software is licensed under project terms.\n\n```markdown\n## License\n```\n\n<!-- License -->\n",
    )
    .expect("README");
    fs::write(root.join("docs/guide.md"), "# License\n").expect("guide");
    fs::write(root.join("LICENSE"), "test fixture license").expect("license");

    let snapshot = seiri_report::audit_repository(&root).expect("audit");
    let route = snapshot
        .route_content
        .assessments
        .iter()
        .find(|item| item.code == "license.readme_route")
        .expect("license README route slot");
    assert!(matches!(route.observation, Observation::Absent { .. }));
    let local_file = snapshot
        .route_content
        .assessments
        .iter()
        .find(|item| item.code == "license.local_file")
        .expect("license file slot");
    assert!(matches!(
        local_file.observation,
        Observation::Present { .. }
    ));
    fs::remove_dir_all(root).expect("cleanup");
}

fn temporary_repository(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-aa-{label}-{nonce}"));
    fs::create_dir_all(&root).expect("temp repository");
    root
}
