use seiri_core::{
    ReadmeRouteMapEntry, ReadmeRouteTargetStatus, RouteKind, RouteSource, RouteState, SourceSpan,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn markdown_extracts_headings_links_badges_and_routes() {
    let summary = seiri_markdown::analyze_readme(fixture("readme-route-repo"))
        .expect("read README")
        .expect("README exists");

    assert_eq!(summary.path, "README.md");
    assert!(summary
        .headings
        .iter()
        .any(|heading| heading.text == "Quickstart"));
    assert!(summary
        .links
        .iter()
        .any(|link| link.text == "Documentation" && link.target == "docs/quickstart.md"));
    assert!(summary.badges.iter().any(|badge| badge.alt == "CI"));

    let routes = summary
        .route_candidates
        .iter()
        .map(|candidate| candidate.route)
        .collect::<BTreeSet<_>>();
    assert!(routes.contains(&RouteKind::Docs));
    assert!(routes.contains(&RouteKind::Quickstart));
    assert!(routes.contains(&RouteKind::Support));
    assert!(routes.contains(&RouteKind::Contributing));
    assert!(routes.contains(&RouteKind::Security));
    assert!(routes.contains(&RouteKind::License));
    assert!(routes.contains(&RouteKind::Release));
    assert!(routes.contains(&RouteKind::Automation));

    assert!(summary
        .route_map
        .entries
        .iter()
        .any(|entry| entry.route == RouteKind::Docs && entry.state == RouteState::Verified));
    assert!(summary
        .route_map
        .entries
        .iter()
        .any(|entry| entry.route == RouteKind::Quickstart && entry.state == RouteState::Routed));
    let automation = route_entry(&summary, RouteKind::Automation);
    assert_eq!(automation.state, RouteState::Routed);
    assert!(automation
        .targets
        .iter()
        .all(|target| target.status == ReadmeRouteTargetStatus::External));
}

#[test]
fn readme_route_map_detects_weak_conflicting_overloaded_and_stale_routes() {
    let summary = seiri_markdown::analyze_readme(fixture("readme-route-map-v2-repo"))
        .expect("read README")
        .expect("README exists");

    assert_eq!(
        route_entry(&summary, RouteKind::Docs).state,
        RouteState::Conflicting
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Quickstart).state,
        RouteState::Conflicting
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Support).state,
        RouteState::Weak
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Security).state,
        RouteState::Stale
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Release).state,
        RouteState::Overloaded
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Governance).state,
        RouteState::Verified
    );

    assert_eq!(
        route_entry(&summary, RouteKind::Docs)
            .gap_estimate
            .expect("docs gap estimate")
            .estimated_repositories,
        186_000
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Quickstart)
            .gap_estimate
            .expect("quickstart gap estimate")
            .estimated_repositories,
        438_000
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Release)
            .gap_estimate
            .expect("release gap estimate")
            .estimated_repositories,
        454_000
    );
}

#[test]
fn readme_route_map_detects_hygiene_self_audit_route() {
    let summary = seiri_markdown::analyze_readme(fixture("hygiene-self-audit-repo"))
        .expect("read README")
        .expect("README exists");

    let hygiene = route_entry(&summary, RouteKind::Hygiene);
    assert_eq!(hygiene.state, RouteState::Verified);
    assert_eq!(hygiene.candidate_count, 1);
    assert_eq!(hygiene.target_count, 1);
}

#[test]
fn q12_external_mail_anchor_and_unknown_targets_are_routed_not_verified() {
    let summary = seiri_markdown::parse_readme(
        "README.md",
        "# Routes\n\n- [Documentation](https://example.invalid/docs)\n- [Support](mailto:help@example.invalid)\n- [Security](#security)\n- [Contributing](CONTRIBUTING.md)\n",
    );

    for route in [
        RouteKind::Docs,
        RouteKind::Support,
        RouteKind::Security,
        RouteKind::Contributing,
    ] {
        assert_eq!(route_entry(&summary, route).state, RouteState::Routed);
    }
    assert!(summary.route_map.entries.iter().all(|entry| {
        entry.state != RouteState::Verified
            || entry
                .targets
                .iter()
                .any(|target| target.status == ReadmeRouteTargetStatus::LocalPresent)
    }));
}

#[test]
fn q12_legacy_readme_gap_key_deserializes_as_an_estimate() {
    let summary = seiri_markdown::analyze_readme(fixture("readme-route-repo"))
        .expect("read README")
        .expect("README exists");
    let docs = route_entry(&summary, RouteKind::Docs);
    let mut json = serde_json::to_value(docs).expect("route entry JSON");
    let object = json.as_object_mut().expect("route entry object");
    object.remove("gap_estimate");
    object.insert("observed_gap_count".to_string(), serde_json::json!(186_000));

    let legacy: ReadmeRouteMapEntry = serde_json::from_value(json).expect("legacy route map entry");
    assert_eq!(
        legacy
            .gap_estimate
            .expect("legacy gap estimate")
            .estimated_repositories,
        186_000
    );
}

#[test]
fn q7_markdown_spans_survive_utf8_multibyte_text() {
    let root = fixture("markdown-span-repo");
    let readme_text = fs::read_to_string(root.join("README.md")).expect("read fixture README");
    let summary = seiri_markdown::analyze_readme(&root)
        .expect("read README")
        .expect("README exists");

    let heading = summary
        .headings
        .iter()
        .find(|heading| heading.text == "RepoSeiri 概要")
        .expect("multibyte heading");
    let heading_span = heading.span.expect("heading span");
    assert_eq!(heading.line, 1);
    assert_eq!(heading_span.line, heading.line);
    assert_eq!(heading_span.column, 1);
    assert_span_slice(&readme_text, heading_span, "# RepoSeiri 概要");

    let quickstart = summary
        .headings
        .iter()
        .find(|heading| heading.text == "Quickstart 手順")
        .expect("route heading");
    let quickstart_span = quickstart.span.expect("quickstart heading span");
    assert_span_slice(&readme_text, quickstart_span, "## Quickstart 手順");

    let link = summary
        .links
        .iter()
        .find(|link| link.target == "docs/guide.md")
        .expect("docs link");
    let link_span = link.span.expect("link span");
    assert_eq!(link_span.line, link.line);
    assert_eq!(link_span.column, expected_column(&readme_text, link_span));
    assert_span_slice(&readme_text, link_span, "[ドキュメント](docs/guide.md)");
    assert!(
        link_span.byte_end - link_span.byte_start > "[ドキュメント](docs/guide.md)".chars().count()
    );

    let badge = summary
        .badges
        .iter()
        .find(|badge| badge.alt == "CI状態")
        .expect("CI badge");
    let badge_span = badge.span.expect("badge span");
    assert_eq!(badge_span.line, badge.line);
    assert_span_slice(
        &readme_text,
        badge_span,
        "![CI状態](https://example.com/badge.svg)",
    );

    assert!(summary
        .route_candidates
        .iter()
        .all(|candidate| candidate.span.is_some()));
    assert_eq!(
        summary
            .route_candidates
            .iter()
            .find(|candidate| {
                candidate.route == RouteKind::Quickstart
                    && candidate.source == RouteSource::Heading
                    && candidate.text == "Quickstart 手順"
            })
            .expect("quickstart candidate")
            .span,
        Some(quickstart_span)
    );
    assert_eq!(
        summary
            .route_candidates
            .iter()
            .find(|candidate| {
                candidate.route == RouteKind::Docs
                    && candidate.source == RouteSource::Link
                    && candidate.target.as_deref() == Some("docs/guide.md")
            })
            .expect("docs candidate")
            .span,
        Some(link_span)
    );
    assert_eq!(
        summary
            .route_candidates
            .iter()
            .find(|candidate| {
                candidate.route == RouteKind::Automation
                    && candidate.source == RouteSource::Badge
                    && candidate.text == "CI状態"
            })
            .expect("badge candidate")
            .span,
        Some(badge_span)
    );

    let json = serde_json::to_value(&summary).expect("summary JSON");
    assert!(json["headings"][0].get("span").is_some());
}

fn route_entry(summary: &seiri_core::ReadmeSummary, route: RouteKind) -> &ReadmeRouteMapEntry {
    summary
        .route_map
        .entries
        .iter()
        .find(|entry| entry.route == route)
        .expect("route map entry")
}

fn assert_span_slice(source: &str, span: SourceSpan, expected: &str) {
    assert_eq!(&source[span.byte_start..span.byte_end], expected);
}

fn expected_column(source: &str, span: SourceSpan) -> usize {
    let line_start = source[..span.byte_start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    source[line_start..span.byte_start].chars().count() + 1
}
