use seiri_core::{
    CalibrationReviewStatus, CalibrationScale, CalibrationSourceKind, ProfileKind, RouteKind,
    SCHEMA_VERSION,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn calibration_ingests_dataset_and_keeps_unmapped_patterns_pending() {
    let dataset =
        seiri_calibration::load_dataset(fixture("calibration-dataset.json")).expect("load dataset");
    let run = seiri_calibration::calibrate_dataset(&dataset);

    assert_eq!(dataset.schema_version, SCHEMA_VERSION);
    assert_eq!(run.schema_version, SCHEMA_VERSION);
    assert_eq!(run.summary.records, 4);
    assert_eq!(run.summary.sources, 1);
    assert_eq!(run.sources[0].kind, CalibrationSourceKind::Fixture);
    assert_eq!(run.sources[0].scale, CalibrationScale::Tiny);

    let docs = run
        .stats
        .iter()
        .find(|stat| stat.pattern_id == "common.docs.route_present")
        .expect("docs stats");
    assert_eq!(docs.route, Some(RouteKind::Docs));
    assert_eq!(docs.repositories, 4);
    assert_eq!(docs.frequency_x1000, 1000);
    assert_eq!(docs.review_status, CalibrationReviewStatus::PendingReview);
    assert!(docs
        .co_occurrences
        .iter()
        .any(|co| co.pattern_id == "common.license.file_present"));

    let docs_requirement = run
        .route_requirements
        .iter()
        .find(|requirement| requirement.route == RouteKind::Docs)
        .expect("docs route requirement");
    assert_eq!(docs_requirement.supporting_repositories, 4);
    assert_eq!(
        docs_requirement.review_status,
        CalibrationReviewStatus::PendingReview
    );

    assert!(run
        .pending_patterns
        .iter()
        .any(
            |candidate| candidate.raw_label == "community.health.code_of_conduct"
                && candidate.review_status == CalibrationReviewStatus::PendingReview
        ));

    assert!(!run.weight_suggestions.is_empty());
    assert!(run
        .weight_suggestions
        .iter()
        .all(|suggestion| suggestion.review_status == CalibrationReviewStatus::PendingReview));
    assert!(run
        .profile_branches
        .iter()
        .any(|branch| branch.profile == ProfileKind::Library));
    assert!(run.claim_boundary.review_required);
    assert!(!run.claim_boundary.runtime_rule_adoption_allowed);
    assert!(run
        .claim_boundary
        .summary
        .contains("does not automatically adopt"));
}

#[test]
fn calibration_uses_profile_hints_for_weight_suggestions() {
    let dataset =
        seiri_calibration::load_dataset(fixture("calibration-dataset.json")).expect("load dataset");
    let run = seiri_calibration::calibrate_dataset(&dataset);

    let library_docs = run
        .weight_suggestions
        .iter()
        .find(|suggestion| {
            suggestion.profile == ProfileKind::Library
                && suggestion.pattern_id == "common.docs.route_present"
        })
        .expect("library docs suggestion");

    assert_eq!(library_docs.frequency_x1000, 1000);
    assert_eq!(library_docs.support_repositories, 1);
    assert_eq!(library_docs.route, Some(RouteKind::Docs));
    assert_eq!(
        library_docs.review_status,
        CalibrationReviewStatus::PendingReview
    );
    assert!(library_docs.rationale.contains("Reviewable calibration"));
}

#[test]
fn calibration_jsonl_loader_wraps_records_with_dataset_metadata() {
    let dataset =
        seiri_calibration::load_dataset(fixture("calibration-records.jsonl")).expect("load jsonl");
    let run = seiri_calibration::calibrate_dataset(&dataset);

    assert_eq!(dataset.dataset_id, "calibration-records");
    assert_eq!(dataset.records.len(), 2);
    assert_eq!(dataset.calibration_sources.len(), 1);
    assert_eq!(
        dataset.calibration_sources[0].kind,
        CalibrationSourceKind::JsonlRecords
    );
    assert_eq!(dataset.evidence_schema.schema_version, SCHEMA_VERSION);
    assert!(run
        .pending_patterns
        .iter()
        .any(|candidate| candidate.raw_label == "install_matrix_table"));
}

#[test]
fn calibration_report_renders_json_and_markdown() {
    let run = seiri_report::calibrate_dataset_path(fixture("calibration-dataset.json"))
        .expect("calibration report");
    let json = seiri_report::calibration_to_json(&run).expect("render calibration JSON");
    let markdown = seiri_report::calibration_to_markdown(&run);

    assert!(json.contains("\"schema_version\": \"seiri.block_p.v1\""));
    assert!(json.contains("\"sources\""));
    assert!(json.contains("\"route_requirements\""));
    assert!(json.contains("\"profile_branches\""));
    assert!(json.contains("\"claim_boundary\""));
    assert!(json.contains("\"pending_patterns\""));
    assert!(markdown.contains("# RepoSeiri Calibration Report"));
    assert!(markdown.contains("## Calibration Sources"));
    assert!(markdown.contains("## Pattern Stats"));
    assert!(markdown.contains("## Route Requirements"));
    assert!(markdown.contains("## Profile Branches"));
    assert!(markdown.contains("## Pending Pattern Candidates"));
    assert!(markdown.contains("## Weight Suggestions"));
}
