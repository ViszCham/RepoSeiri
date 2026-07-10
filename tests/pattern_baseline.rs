use seiri_core::{
    BaselineRequirement, BaselineStatus, PatternGroup, PatternOutcome, RouteKind, RouteState,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn common_registry_exposes_stable_pattern_definitions() {
    let registry = seiri_patterns::common_registry();
    let ids = registry
        .definitions()
        .iter()
        .map(|definition| definition.id)
        .collect::<Vec<_>>();

    assert!(ids.contains(&"common.identity.readme_present"));
    assert!(ids.contains(&"common.docs.route_present"));
    assert!(ids.contains(&"common.quickstart.route_present"));
    assert!(ids.contains(&"common.license.file_present"));
    assert!(ids.contains(&"common.lifecycle.route_present"));
    assert!(ids.contains(&"SEC-001"));
    assert!(ids.contains(&"SEC-004"));
    assert!(ids.contains(&"SEC-007"));
    assert!(ids.contains(&"SUP-001"));
    assert!(ids.contains(&"INT-001"));
    assert!(ids.contains(&"INT-002"));
    assert!(ids.contains(&"INT-003"));
    assert!(ids.contains(&"INT-010"));
    assert!(ids.contains(&"AUT-001"));
    assert!(ids.contains(&"AUT-009"));
    assert!(ids.contains(&"REL-002"));
    assert!(ids.contains(&"LIF-001"));
    assert!(ids.contains(&"OWN-001"));

    assert!(registry.definitions().iter().any(|definition| {
        definition.id == "common.docs.route_present" && definition.group == PatternGroup::Doc
    }));
    assert!(registry
        .definitions()
        .iter()
        .any(|definition| { definition.id == "SEC-001" && definition.group == PatternGroup::Sec }));
    assert_eq!(registry.evaluation_definitions().len(), 10);
    assert!(registry.evaluation_definitions().iter().all(|definition| {
        definition.adoption_stage == seiri_patterns::PatternAdoptionStage::CommonBaseline
    }));
    registry
        .validate_complete()
        .expect("common registry coverage");
}

#[test]
fn pattern_registry_v3_renders_grouped_json_and_markdown() {
    let registry = seiri_patterns::common_registry();
    let document = seiri_patterns::registry_document(&registry);

    assert_eq!(document.schema_version, seiri_core::SCHEMA_VERSION);
    assert_eq!(document.registry_version, "pattern_registry.v3");
    assert_eq!(document.groups.len(), 13);
    assert_eq!(document.negative_fixtures.len(), 13);
    assert!(document
        .groups
        .iter()
        .all(|group| { group.detector_count > 0 && group.negative_fixture_count > 0 }));
    assert!(document
        .groups
        .iter()
        .any(|group| group.code == "SEC" && group.pattern_count >= 2));
    assert!(document
        .patterns
        .iter()
        .any(|pattern| pattern.id == "SEC-001" && !pattern.active_in_common_baseline));
    assert!(document
        .patterns
        .iter()
        .any(|pattern| pattern.id == "INT-003" && pattern.route == Some(RouteKind::Intake)));

    let json = seiri_patterns::registry_to_json(&registry).expect("registry json");
    assert!(json.contains("\"registry_version\": \"pattern_registry.v3\""));
    assert!(json.contains("\"negative_fixtures\""));
    assert!(json.contains("\"group\": \"SEC\""));
    assert!(json.contains("\"id\": \"OWN-001\""));

    let markdown = seiri_patterns::render_registry_markdown(&registry);
    assert!(markdown.contains("## Negative Fixtures"));
    assert!(markdown.contains("`negative.hyg.minimal` group `HYG`"));
    assert!(markdown.contains("## SEC - Security"));
    assert!(markdown.contains("`SEC-001` `candidate`"));
    assert!(markdown.contains("## HYG - Hygiene"));
}

#[test]
fn common_baseline_marks_route_fixture_as_complete_for_required_rules() {
    let snapshot =
        seiri_report::audit_repository(fixture("readme-route-repo")).expect("audit fixture");
    let baseline = snapshot.baseline.expect("baseline report");

    assert_eq!(baseline.summary.required_missing, 0);
    assert!(baseline.summary.required_present >= 4);
    assert!(baseline
        .rules
        .iter()
        .filter(|rule| rule.requirement == BaselineRequirement::Required)
        .all(|rule| rule.status == BaselineStatus::Present));
    assert!(snapshot
        .pattern_matches
        .iter()
        .all(|pattern_match| pattern_match.outcome == PatternOutcome::Present));
}

#[test]
fn common_baseline_generates_findings_from_missing_patterns() {
    let snapshot =
        seiri_report::audit_repository(fixture("missing-readme-repo")).expect("audit fixture");
    let baseline = snapshot.baseline.expect("baseline report");

    assert!(baseline.summary.required_missing >= 3);
    assert!(snapshot
        .pattern_matches
        .iter()
        .any(
            |pattern_match| pattern_match.pattern_id == "common.identity.readme_present"
                && pattern_match.outcome == PatternOutcome::Missing
        ));
    assert!(snapshot
        .findings
        .iter()
        .any(|finding| finding.title == "README is missing"));
    assert!(baseline
        .rules
        .iter()
        .any(|rule| rule.status == BaselineStatus::Missing && rule.finding_id.is_some()));
}

#[test]
fn common_baseline_does_not_credit_nested_license_as_root_license() {
    let snapshot =
        seiri_report::audit_repository(fixture("nested-license-only-repo")).expect("audit fixture");
    let baseline = snapshot.baseline.expect("baseline report");

    assert!(baseline.rules.iter().any(|rule| {
        rule.pattern_id == "common.license.file_present" && rule.status == BaselineStatus::Missing
    }));
    assert!(snapshot.pattern_matches.iter().any(|pattern_match| {
        pattern_match.pattern_id == "common.license.file_present"
            && pattern_match.outcome == PatternOutcome::Missing
    }));
    assert!(snapshot.route_states.iter().any(|state| {
        state.route == RouteKind::License && state.state == RouteState::Inherited
    }));
}
