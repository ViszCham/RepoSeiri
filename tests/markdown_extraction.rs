use seiri_core::RouteKind;
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
}
