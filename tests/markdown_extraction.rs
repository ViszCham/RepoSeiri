use seiri_core::{ReadmeRouteMapEntry, RouteKind, RouteState};
use std::collections::BTreeSet;
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
        route_entry(&summary, RouteKind::Docs).observed_gap_count,
        Some(186_000)
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Quickstart).observed_gap_count,
        Some(438_000)
    );
    assert_eq!(
        route_entry(&summary, RouteKind::Release).observed_gap_count,
        Some(454_000)
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

fn route_entry(summary: &seiri_core::ReadmeSummary, route: RouteKind) -> &ReadmeRouteMapEntry {
    summary
        .route_map
        .entries
        .iter()
        .find(|entry| entry.route == route)
        .expect("route map entry")
}
