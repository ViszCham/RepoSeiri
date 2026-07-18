use seiri_report::{
    evaluate_public_holdout, EmpiricalCalibrationStatus, PeakAllocationMeasurement,
    HOLDOUT_REPORT_SCHEMA_VERSION, HOLDOUT_SPLIT_METHOD, MINIMUM_HOLDOUT_CASES_PER_TASK,
};
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn public_holdout_evaluates_all_five_tasks_without_claim_promotion() {
    let report = evaluate_public_holdout(
        root().join("fixtures/calibration-holdout-corpus.v1.json"),
        root().join("fixtures"),
    )
    .expect("public holdout");
    assert_eq!(report.schema_version, HOLDOUT_REPORT_SCHEMA_VERSION);
    assert_eq!(
        report.status,
        EmpiricalCalibrationStatus::InsufficientSample
    );
    assert_eq!(report.task_metrics.len(), 5);
    assert_eq!(report.private_overlay, "not_included");
    assert_eq!(report.split_method, HOLDOUT_SPLIT_METHOD);
    assert!(report
        .boundary
        .contains("do not establish general performance"));

    for metric in &report.task_metrics {
        assert_eq!(metric.train.samples(), 2);
        assert_eq!(metric.holdout.samples(), 4);
        assert_eq!(metric.independent_holdout_cases, 4);
        assert_eq!(metric.holdout.true_positive, 2, "{:?}", metric.task);
        assert_eq!(metric.holdout.true_negative, 2, "{:?}", metric.task);
        assert_eq!(metric.false_positives, 0, "{:?}", metric.task);
        assert_eq!(metric.false_negatives, 0, "{:?}", metric.task);
        assert_eq!(metric.precision_x1000, Some(1000));
        assert_eq!(metric.recall_x1000, Some(1000));
        assert_eq!(metric.coverage_x1000, 1000);
        assert_eq!(metric.minimum_holdout_cases, MINIMUM_HOLDOUT_CASES_PER_TASK);
        assert_eq!(
            metric.status,
            EmpiricalCalibrationStatus::InsufficientSample
        );
        assert!(matches!(
            metric.resources.peak_allocation,
            PeakAllocationMeasurement::NotMeasured { .. }
        ));
    }

    let json = serde_json::to_string_pretty(&report).expect("holdout JSON");
    let value: serde_json::Value = serde_json::from_str(&json).expect("holdout JSON value");
    assert_eq!(
        value["task_metrics"][0]["accuracy_interval"]["method"],
        "wilson_95"
    );
    let absolute = root().to_string_lossy().replace('\\', "/");
    assert!(!json.contains(&absolute));
    assert!(!json.contains("holdout-wording-visible"));
    assert!(!json.contains("private-calibration"));
}

#[test]
fn corpus_digest_is_stable_while_runtime_remains_measurement_only() {
    let corpus = root().join("fixtures/calibration-holdout-corpus.v1.json");
    let fixtures = root().join("fixtures");
    let first = evaluate_public_holdout(&corpus, &fixtures).expect("first report");
    let second = evaluate_public_holdout(&corpus, &fixtures).expect("second report");
    assert_eq!(first.corpus_digest, second.corpus_digest);
    assert_eq!(first.corpus_id, second.corpus_id);
    assert_eq!(first.task_metrics.len(), second.task_metrics.len());
}

#[test]
fn corpus_reader_rejects_oversized_and_invalid_inputs_without_path_disclosure() {
    let temporary = tempfile::tempdir().expect("temporary corpus");
    let temporary_path = temporary.path().to_string_lossy().into_owned();
    let oversized = temporary.path().join("oversized.json");
    fs::write(&oversized, vec![b'x'; 1024 * 1024 + 1]).expect("oversized corpus");
    let error =
        evaluate_public_holdout(&oversized, root().join("fixtures")).expect_err("oversized corpus");
    assert!(error.to_string().contains("byte limit"));
    assert!(!error.to_string().contains(&temporary_path));

    let invalid = temporary.path().join("invalid.json");
    fs::write(&invalid, b"{}").expect("invalid corpus");
    let error =
        evaluate_public_holdout(&invalid, root().join("fixtures")).expect_err("invalid corpus");
    assert!(error.to_string().contains("invariants"));
    assert!(!error.to_string().contains(&temporary_path));

    let source = fs::read(root().join("fixtures/calibration-holdout-corpus.v1.json"))
        .expect("source corpus");
    let mut reused: serde_json::Value =
        serde_json::from_slice(&source).expect("source corpus JSON");
    reused["cases"][2]["fixture"] = serde_json::json!("readme-route-repo");
    let reused_path = temporary.path().join("reused-fixture.json");
    fs::write(
        &reused_path,
        serde_json::to_vec_pretty(&reused).expect("reused corpus JSON"),
    )
    .expect("reused corpus");
    let error = evaluate_public_holdout(&reused_path, root().join("fixtures"))
        .expect_err("train/holdout fixture reuse");
    assert!(error.to_string().contains("invariants"));
    assert!(!error.to_string().contains(&temporary_path));
}
