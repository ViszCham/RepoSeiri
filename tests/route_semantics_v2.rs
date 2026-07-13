use seiri_core::{ProfileKind, RouteAvailability, RouteKind, RouteState};
use std::fs;

#[test]
fn bilingual_duplicate_routes_have_one_logical_route_per_target() {
    let root = tempfile::tempdir().expect("repository");
    fs::create_dir_all(root.path().join("docs")).expect("docs");
    fs::write(root.path().join("docs/guide.md"), "# Guide\n").expect("guide");
    fs::write(
        root.path().join("README.md"),
        "# Tool\n\n## Docs\n[ドキュメント](docs/guide.md)\n\n## Documentation\n[Documentation](./docs/guide.md)\n",
    )
    .expect("README");

    let snapshot = seiri_report::audit_repository(root.path()).expect("audit");
    let route = snapshot
        .readme_summary
        .as_ref()
        .expect("README")
        .route_map
        .entries
        .iter()
        .find(|entry| entry.route == RouteKind::Docs)
        .expect("docs route");
    assert_eq!(route.raw_candidate_count, 4);
    assert_eq!(route.candidate_count, 2);
    assert_ne!(route.state, RouteState::Overloaded);
}

#[test]
fn degraded_route_is_not_a_missing_co_occurrence_member() {
    let root = tempfile::tempdir().expect("repository");
    fs::write(root.path().join("SECURITY.md"), "# Security\n").expect("security");
    fs::write(
        root.path().join("README.md"),
        "# Tool\n\n[CI A](https://example.com/a)\n[CI B](https://example.com/b)\n[CI C](https://example.com/c)\n[CI D](https://example.com/d)\n[Security](SECURITY.md)\n",
    )
    .expect("README");

    let snapshot = seiri_report::audit_repository_with_profile(root.path(), ProfileKind::Common)
        .expect("audit");
    let automation = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::Automation)
        .expect("automation route");
    assert_eq!(
        automation.condition().availability,
        RouteAvailability::Degraded
    );
    assert_eq!(automation.condition().state, Some(RouteState::Overloaded));

    let gap = snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .iter()
        .find(|gap| gap.id == "co-README-SECURITY-CI-DEPENDENCY-BOT")
        .expect("supply-chain gap");
    assert!(gap.degraded_routes.contains(&RouteKind::Automation));
    assert!(!gap.missing_routes.contains(&RouteKind::Automation));
    assert!(gap.unknown_routes.is_empty());
}

#[test]
fn route_presence_and_condition_are_separate_v2_fields() {
    let root = tempfile::tempdir().expect("repository");
    fs::write(root.path().join("README.md"), "# Tool\n\n## Support\n").expect("README");
    let snapshot = seiri_report::audit_repository(root.path()).expect("audit");
    let support = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::Support)
        .expect("support");
    assert!(!support.presence().root_structured());
    assert!(support.readme().routing().is_present());
    assert_eq!(
        support.condition().availability,
        RouteAvailability::Degraded
    );

    let value = serde_json::to_value(support).expect("route wire");
    assert!(value.get("presence").is_some());
    assert!(value.get("condition").is_some());
}

#[test]
fn planner_does_not_hold_or_patch_an_existing_inline_route() {
    let root = tempfile::tempdir().expect("repository");
    fs::create_dir_all(root.path().join("docs")).expect("docs");
    fs::write(root.path().join("docs/index.md"), "# Guide\n").expect("guide");
    fs::write(
        root.path().join("README.md"),
        "# Tool\n\n## Documentation\n",
    )
    .expect("README");

    let plan = seiri_report::plan_repository(root.path()).expect("plan");
    assert!(!plan
        .operations
        .iter()
        .any(|item| item.route == RouteKind::Docs));
    assert!(!plan.held.iter().any(|item| item.route == RouteKind::Docs));
}
