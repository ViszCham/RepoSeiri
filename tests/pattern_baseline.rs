use seiri_core::{BaselineRequirement, BaselineStatus, PatternOutcome};
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
