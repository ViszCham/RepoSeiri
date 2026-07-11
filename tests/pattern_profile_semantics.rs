use seiri_core::{PatternGroup, ProfileEvidenceBasis, ProfileKind, ProfileWeightBasis};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn every_pattern_group_has_an_executable_negative_fixture() {
    let registry = seiri_patterns::common_registry();

    for group in PatternGroup::ALL {
        assert!(
            registry
                .definitions()
                .iter()
                .any(|definition| definition.group == group),
            "{group} detector coverage"
        );

        let fixtures = registry.negative_fixtures_for(group).collect::<Vec<_>>();
        assert!(!fixtures.is_empty(), "{group} negative fixture coverage");
        for negative in fixtures {
            let repository = fixture(negative.repository);
            assert!(repository.is_dir(), "registered fixture must exist");
            let snapshot =
                seiri_report::audit_repository_subtree(&repository).expect("audit fixture");
            let definition = registry
                .definition(negative.pattern_id)
                .expect("fixture pattern must be registered");
            let evidence_ids = seiri_patterns::evidence_ids_for_definition(&snapshot, definition);
            assert!(
                evidence_ids.is_empty(),
                "fixture '{}' must be negative for '{}'",
                negative.id,
                negative.pattern_id
            );
        }
    }
}

#[test]
fn complete_pattern_registry_rejects_missing_group_fixture_coverage() {
    let registry = seiri_patterns::common_registry();
    let fixtures = registry
        .negative_fixtures()
        .iter()
        .copied()
        .filter(|fixture| fixture.group != PatternGroup::Hyg)
        .collect();

    let error =
        seiri_patterns::PatternRegistry::try_complete(registry.definitions().to_vec(), fixtures)
            .expect_err("missing HYG fixture must fail");
    assert_eq!(
        error,
        seiri_patterns::PatternRegistryError::MissingNegativeFixture(PatternGroup::Hyg)
    );
}

#[test]
fn complete_pattern_registry_rejects_missing_group_detector_coverage() {
    let registry = seiri_patterns::common_registry();
    let definitions = registry
        .definitions()
        .iter()
        .filter(|definition| definition.group != PatternGroup::Gov)
        .cloned()
        .collect();
    let fixtures = registry
        .negative_fixtures()
        .iter()
        .copied()
        .filter(|fixture| fixture.group != PatternGroup::Gov)
        .collect();

    let error = seiri_patterns::PatternRegistry::try_complete(definitions, fixtures)
        .expect_err("missing GOV detector must fail");
    assert_eq!(
        error,
        seiri_patterns::PatternRegistryError::MissingDetector(PatternGroup::Gov)
    );
}

#[test]
fn profile_registry_is_complete_and_uses_nonzero_static_weights() {
    let registry = seiri_profiles::common_profile_registry();
    registry
        .validate_complete()
        .expect("profile registry coverage");
    assert_eq!(registry.definitions().len(), ProfileKind::ALL.len());
    assert!(registry.definitions().iter().all(|definition| {
        !definition.rules.is_empty() && definition.rules.iter().all(|rule| rule.weight.get() > 0)
    }));
}

#[test]
fn calibration_suggestions_do_not_mutate_profile_scores() {
    let repository = fixture("docs-routed-repo");
    let before =
        seiri_report::audit_repository_subtree_with_profile(&repository, ProfileKind::Library)
            .expect("audit before calibration")
            .profile
            .expect("profile before calibration");

    let dataset = seiri_calibration::load_dataset(fixture("calibration-dataset.json"))
        .expect("load calibration fixture");
    let calibration = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");
    assert!(!calibration.weight_suggestions.is_empty());

    let after =
        seiri_report::audit_repository_subtree_with_profile(&repository, ProfileKind::Library)
            .expect("audit after calibration")
            .profile
            .expect("profile after calibration");

    assert_eq!(before.score, after.score);
    assert_eq!(
        after.score.evidence_basis,
        ProfileEvidenceBasis::RepositoryEvidence
    );
    assert_eq!(
        after.score.weight_basis,
        ProfileWeightBasis::StaticProfileRegistry
    );
    assert!(after.score.note.contains("review-only suggestions"));
}

#[test]
fn profile_score_does_not_credit_present_status_without_evidence() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit route fixture");
    let mut baseline = snapshot.baseline.expect("baseline report");
    let identity = baseline
        .rules
        .iter_mut()
        .find(|rule| rule.pattern_id == "common.identity.readme_present")
        .expect("identity rule");
    assert_eq!(identity.status, seiri_core::BaselineStatus::Present);
    identity.evidence_ids.clear();

    let profile = seiri_profiles::evaluate_profile_from_parts(
        &baseline,
        &snapshot.findings,
        ProfileKind::Common,
    );
    let identity = profile
        .rules
        .iter()
        .find(|rule| rule.pattern_id == "common.identity.readme_present")
        .expect("profile identity rule");

    assert_eq!(identity.status, seiri_core::BaselineStatus::Missing);
    assert!(profile.score.earned_weight < profile.score.total_weight);
}
