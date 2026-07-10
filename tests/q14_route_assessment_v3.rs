use seiri_core::{
    ReadmeRouteMapEntry, ReadmeRouteTargetStatus, RouteFreshness, RouteKind, RouteState,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn q14_readme_assessment_keeps_axes_before_legacy_projection() {
    let summary = seiri_markdown::analyze_readme(fixture("readme-route-map-v2-repo"))
        .expect("read README")
        .expect("README exists");

    let docs = route_entry(&summary, RouteKind::Docs);
    assert_eq!(docs.assessment.routing().candidate_count(), 1);
    assert_eq!(docs.assessment.routing().target_count(), 1);
    assert_eq!(docs.assessment.conflict().shared_target_count(), 1);
    assert_eq!(docs.assessment.freshness(), RouteFreshness::Current);
    assert_eq!(
        docs.assessment.legacy_state(RouteKind::Docs),
        RouteState::Conflicting
    );
    assert_eq!(docs.state, RouteState::Conflicting);

    let security = route_entry(&summary, RouteKind::Security);
    assert_eq!(
        security
            .assessment
            .target_reachability()
            .repository_local_missing(),
        1
    );
    assert_eq!(security.assessment.freshness(), RouteFreshness::Stale);
    assert_eq!(security.state, RouteState::Stale);

    let release = route_entry(&summary, RouteKind::Release);
    assert!(release.assessment.routing().is_overloaded());
    assert_eq!(release.state, RouteState::Overloaded);

    let support = route_entry(&summary, RouteKind::Support);
    assert!(support.assessment.routing().is_present());
    assert_eq!(support.assessment.routing().target_count(), 0);
    assert_eq!(support.state, RouteState::Weak);
}

#[test]
fn q14_repository_assessment_projects_every_legacy_route_state() {
    let snapshot = seiri_report::audit_repository(fixture("readme-route-map-v2-repo"))
        .expect("audit route fixture");

    assert_eq!(
        snapshot.route_assessments.len(),
        snapshot.route_states.len()
    );
    assert!(!snapshot.route_assessments.is_empty());
    let kernel_ids = snapshot
        .evidence_kernel
        .facts()
        .iter()
        .map(|fact| fact.id)
        .collect::<BTreeSet<_>>();

    for assessment in &snapshot.route_assessments {
        let state = snapshot
            .route_states
            .iter()
            .find(|state| state.route == assessment.route())
            .expect("legacy projection");
        let projection = assessment.legacy_projection();
        assert_eq!(state.state, projection.state);
        assert_eq!(state.confidence, projection.confidence);
        assert_eq!(state.reason, projection.reason);
        assert_eq!(state.evidence_ids, assessment.legacy_evidence_ids());
        assert!(assessment
            .evidence()
            .root_structural()
            .iter()
            .chain(assessment.evidence().readme_routing())
            .chain(assessment.evidence().inherited())
            .all(|id| kernel_ids.contains(id)));

        let json = serde_json::to_value(assessment).expect("serialize route assessment");
        assert!(json.get("state").is_none());
        assert!(json.get("presence").is_some());
        assert!(json["readme"].get("target_reachability").is_some());
        assert!(json["readme"].get("conflict").is_some());
        assert!(json["readme"].get("freshness").is_some());
    }

    let report_json = seiri_report::to_json(&snapshot).expect("snapshot JSON");
    let report: serde_json::Value =
        serde_json::from_str(&report_json).expect("parse snapshot JSON");
    assert!(report["route_assessments"].is_array());
    assert!(report["route_states"].is_array());
    let markdown = seiri_report::to_markdown(&snapshot);
    assert!(markdown.contains("## Route Assessment Axes"));
    assert!(markdown.contains("freshness"));
}

#[test]
fn q14_native_wire_rejects_inconsistent_freshness_and_projection() {
    let summary = seiri_markdown::analyze_readme(fixture("readme-route-map-v2-repo"))
        .expect("read README")
        .expect("README exists");
    let security = route_entry(&summary, RouteKind::Security);

    let mut assessment_json = serde_json::to_value(security.assessment).expect("assessment JSON");
    assessment_json["freshness"] = serde_json::json!("current");
    assert!(serde_json::from_value::<seiri_core::ReadmeRouteAssessment>(assessment_json).is_err());

    let mut count_json = serde_json::to_value(security.assessment).expect("assessment JSON");
    count_json["routing"]["candidate_count"] = serde_json::json!(2);
    assert!(serde_json::from_value::<seiri_core::ReadmeRouteAssessment>(count_json).is_err());

    let mut entry_json = serde_json::to_value(security).expect("route entry JSON");
    entry_json["state"] = serde_json::json!("verified");
    assert!(serde_json::from_value::<ReadmeRouteMapEntry>(entry_json).is_err());

    let mut compatibility_count_json = serde_json::to_value(security).expect("route entry JSON");
    compatibility_count_json["stale_target_count"] = serde_json::json!(0);
    assert!(serde_json::from_value::<ReadmeRouteMapEntry>(compatibility_count_json).is_err());

    let snapshot = seiri_report::audit_repository(fixture("readme-route-map-v2-repo"))
        .expect("audit route fixture");
    let aggregate_security = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::Security)
        .expect("security assessment");

    let mut presence_json =
        serde_json::to_value(aggregate_security).expect("aggregate assessment JSON");
    presence_json["presence"]["root_structured"] = serde_json::json!(true);
    assert!(serde_json::from_value::<seiri_core::RouteAssessment>(presence_json).is_err());

    let mut policy_json =
        serde_json::to_value(aggregate_security).expect("aggregate assessment JSON");
    policy_json["policy"] = serde_json::json!("suggestible");
    assert!(serde_json::from_value::<seiri_core::RouteAssessment>(policy_json).is_err());

    let release = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::Release)
        .expect("release assessment");
    let mut evidence_json = serde_json::to_value(release).expect("aggregate assessment JSON");
    let ids = evidence_json["evidence"]["readme_routing"]
        .as_array_mut()
        .expect("README evidence ids");
    assert!(ids.len() > 1);
    ids.reverse();
    assert!(serde_json::from_value::<seiri_core::RouteAssessment>(evidence_json).is_err());
}

#[test]
fn q14_legacy_entry_rebuilds_assessment_and_state_from_observations() {
    let summary = seiri_markdown::analyze_readme(fixture("readme-route-map-v2-repo"))
        .expect("read README")
        .expect("README exists");
    let security = route_entry(&summary, RouteKind::Security);
    let mut legacy_json = serde_json::to_value(security).expect("route entry JSON");
    let object = legacy_json.as_object_mut().expect("route entry object");
    object.remove("assessment");
    object.insert("state".to_string(), serde_json::json!("verified"));

    let rebuilt =
        serde_json::from_value::<ReadmeRouteMapEntry>(legacy_json).expect("legacy route entry");
    assert_eq!(rebuilt.assessment.freshness(), RouteFreshness::Stale);
    assert_eq!(
        rebuilt
            .assessment
            .target_reachability()
            .repository_local_missing(),
        1
    );
    assert_eq!(rebuilt.state, RouteState::Stale);
}

#[test]
fn q14_nonlocal_targets_remain_separate_from_local_reachability() {
    let summary = seiri_markdown::parse_readme(
        "README.md",
        "# Routes\n\n- [Documentation](https://example.invalid/docs)\n- [Support](mailto:help@example.invalid)\n- [Security](#security)\n- [Contributing](CONTRIBUTING.md)\n",
    );

    let docs = route_entry(&summary, RouteKind::Docs);
    assert_eq!(docs.assessment.target_reachability().external(), 1);
    assert_eq!(
        docs.assessment
            .target_reachability()
            .repository_local_present(),
        0
    );
    assert_eq!(docs.assessment.freshness(), RouteFreshness::NotApplicable);

    let support = route_entry(&summary, RouteKind::Support);
    assert_eq!(support.assessment.target_reachability().mail(), 1);
    let security = route_entry(&summary, RouteKind::Security);
    assert_eq!(security.assessment.target_reachability().anchor(), 1);
    let contributing = route_entry(&summary, RouteKind::Contributing);
    assert_eq!(contributing.assessment.target_reachability().unknown(), 1);

    for entry in [docs, support, security, contributing] {
        assert_eq!(entry.state, RouteState::Routed);
        assert!(entry
            .targets
            .iter()
            .all(|target| target.status != ReadmeRouteTargetStatus::LocalPresent));
    }
}

fn route_entry(summary: &seiri_core::ReadmeSummary, route: RouteKind) -> &ReadmeRouteMapEntry {
    summary
        .route_map
        .entries
        .iter()
        .find(|entry| entry.route == route)
        .expect("route map entry")
}
