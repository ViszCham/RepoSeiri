use seiri_core::{
    CalibrationReviewStatus, CalibrationScale, CalibrationSource, CalibrationSourceKind,
    CalibrationSourceVisibility, ProfileKind, RouteKind, CALIBRATION_SCHEMA_VERSION,
    EVIDENCE_SCHEMA_VERSION,
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
    let run = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");

    assert_eq!(dataset.schema_version, CALIBRATION_SCHEMA_VERSION);
    assert_eq!(run.schema_version, CALIBRATION_SCHEMA_VERSION);
    assert_eq!(run.summary.records, 4);
    assert_eq!(run.summary.sources, 1);
    assert_eq!(run.sources[0].kind, CalibrationSourceKind::Fixture);
    assert_eq!(
        run.sources[0].visibility,
        CalibrationSourceVisibility::Public
    );
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
fn calibration_rejects_noncanonical_dataset_and_evidence_schemas() {
    let dataset =
        seiri_calibration::load_dataset(fixture("calibration-dataset.json")).expect("load dataset");

    let mut wrong_dataset = dataset.clone();
    wrong_dataset.schema_version = "seiri.calibration.invalid".to_string();
    assert!(matches!(
        seiri_calibration::calibrate_dataset(&wrong_dataset),
        Err(seiri_calibration::CalibrationError::SchemaMismatch {
            domain: "calibration dataset",
            ..
        })
    ));

    let mut wrong_evidence = dataset.clone();
    wrong_evidence.evidence_schema.schema_version = "seiri.evidence.invalid".to_string();
    assert!(matches!(
        seiri_calibration::calibrate_dataset(&wrong_evidence),
        Err(seiri_calibration::CalibrationError::SchemaMismatch {
            domain: "calibration evidence",
            ..
        })
    ));

    let mut removed_metadata = serde_json::to_value(dataset).expect("dataset JSON");
    removed_metadata["evidence_schema"]["compatible_from"] = serde_json::json!("seiri.evidence.v1");
    assert!(serde_json::from_value::<seiri_core::BenchmarkDataset>(removed_metadata).is_err());
}

#[test]
fn calibration_uses_profile_hints_for_weight_suggestions() {
    let dataset =
        seiri_calibration::load_dataset(fixture("calibration-dataset.json")).expect("load dataset");
    let run = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");

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
    let run = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");

    assert_eq!(dataset.dataset_id, "calibration-records");
    assert_eq!(dataset.records.len(), 2);
    assert_eq!(dataset.calibration_sources.len(), 1);
    assert_eq!(
        dataset.calibration_sources[0].kind,
        CalibrationSourceKind::JsonlRecords
    );
    assert_eq!(
        dataset.calibration_sources[0].visibility,
        CalibrationSourceVisibility::LocalOnly
    );
    assert_eq!(
        dataset.evidence_schema.schema_version,
        EVIDENCE_SCHEMA_VERSION
    );
    assert!(run
        .pending_patterns
        .iter()
        .any(|candidate| candidate.raw_label == "install_matrix_table"));

    let public_json = seiri_report::calibration_to_json(&run).expect("public JSONL report");
    assert!(public_json.contains("\"visibility\": \"redacted\""));
    assert!(!public_json.contains("\"dataset_id\": \"calibration-records\""));
}

#[test]
fn omitted_calibration_visibility_fails_closed() {
    let source: CalibrationSource = serde_json::from_value(serde_json::json!({
        "id": "source-without-visibility",
        "kind": "aggregate_analysis",
        "label": "synthetic source",
        "collected_at": "unknown",
        "records": 12,
        "scale": "tiny",
        "review_status": "pending_review"
    }))
    .expect("deserialize source without visibility");

    assert_eq!(source.visibility, CalibrationSourceVisibility::LocalOnly);
    assert_eq!(
        CalibrationSourceVisibility::default(),
        CalibrationSourceVisibility::LocalOnly
    );
}

#[test]
fn calibration_report_renders_json_and_markdown() {
    let run = seiri_report::calibrate_dataset_path(fixture("calibration-dataset.json"))
        .expect("calibration report");
    let json = seiri_report::calibration_to_json(&run).expect("render calibration JSON");
    let markdown = seiri_report::calibration_to_markdown(&run);

    assert!(json.contains("\"schema_version\": \"seiri.calibration.v2\""));
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

#[test]
fn public_calibration_outputs_redact_local_only_sources() {
    let dataset = seiri_calibration::load_dataset(fixture("calibration-local-only-dataset.json"))
        .expect("load local-only fixture");
    let run = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");

    assert_eq!(
        run.sources[1].visibility,
        CalibrationSourceVisibility::LocalOnly
    );
    assert!(run
        .pending_patterns
        .iter()
        .any(|candidate| candidate.raw_label == "SYNTHETIC_LOCAL_ONLY_BODY_SHOULD_NOT_RENDER"));

    let public_run = run.redacted_for_public_output();
    assert_eq!(public_run.dataset_id, "redacted-calibration-dataset");
    assert_eq!(
        public_run.sources[1].visibility,
        CalibrationSourceVisibility::Redacted
    );
    assert!(public_run
        .pending_patterns
        .iter()
        .all(|candidate| candidate.example_locations.is_empty()));
    assert!(public_run
        .pending_patterns
        .iter()
        .all(|candidate| candidate.raw_label == "redacted local-only pattern candidate"));

    let json = seiri_report::calibration_to_json(&run).expect("render redacted JSON");
    let markdown = seiri_report::calibration_to_markdown(&run);

    assert!(json.contains("\"visibility\": \"redacted\""));
    assert!(json.contains("redacted-calibration-source-0002"));
    assert!(markdown.contains("Source visibility: public `1` / local_only `1` / redacted `0`"));
    assert!(markdown.contains("visibility `Redacted`"));

    assert_no_synthetic_local_details(&json);
    assert_no_synthetic_local_details(&markdown);
}

fn assert_no_synthetic_local_details(output: &str) {
    for token in [
        "SYNTHETIC_LOCAL_ONLY_DATASET_ID_SHOULD_NOT_RENDER",
        "SYNTHETIC_LOCAL_ONLY_PATH_SHOULD_NOT_RENDER",
        "SYNTHETIC_LOCAL_ONLY_BODY_SHOULD_NOT_RENDER",
    ] {
        assert!(
            !output.contains(token),
            "public output leaked synthetic local-only token `{token}`"
        );
    }
}
