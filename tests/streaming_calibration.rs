use seiri_calibration::{
    calibrate_jsonl_path, calibrate_jsonl_reader, calibrate_jsonl_reader_with_limits,
    StreamingCalibrationLimits, StreamingCalibrationMetadata, StreamingLimitKind,
};
use seiri_core::{CalibrationAggregationMode, CalibrationRecordIdentity, CalibrationReplayDigest};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn metadata() -> StreamingCalibrationMetadata {
    StreamingCalibrationMetadata::new("calibration-records", "calibration-records", "unknown")
}

#[test]
fn streaming_jsonl_matches_materialized_results_for_unique_records() {
    let path = fixture("calibration-records.jsonl");
    let dataset = seiri_calibration::load_dataset(&path).expect("materialized dataset");
    let materialized = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");
    let streaming = calibrate_jsonl_path(&path).expect("streaming calibration");

    assert_eq!(streaming.summary, materialized.summary);
    assert_eq!(streaming.stats, materialized.stats);
    assert_eq!(
        streaming.route_requirements,
        materialized.route_requirements
    );
    assert_eq!(streaming.profile_branches, materialized.profile_branches);
    assert_eq!(streaming.pending_patterns, materialized.pending_patterns);
    assert_eq!(
        streaming.weight_suggestions,
        materialized.weight_suggestions
    );
    assert_eq!(
        streaming.resource_trace.aggregation_mode,
        CalibrationAggregationMode::StreamingJsonl
    );
    assert_eq!(
        materialized.resource_trace.aggregation_mode,
        CalibrationAggregationMode::MaterializedDataset
    );
}

#[test]
fn streaming_replay_is_deterministic_and_digest_is_typed() {
    let bytes = fs::read(fixture("calibration-records.jsonl")).expect("fixture bytes");
    let first = calibrate_jsonl_reader(Cursor::new(&bytes), metadata()).expect("first replay");
    let second = calibrate_jsonl_reader(Cursor::new(&bytes), metadata()).expect("second replay");

    assert_eq!(first, second);
    let digest = first
        .resource_trace
        .replay_digest
        .expect("streaming digest");
    let wire = serde_json::to_string(&digest).expect("digest JSON");
    let roundtrip = serde_json::from_str::<CalibrationReplayDigest>(&wire).expect("digest wire");
    assert_eq!(roundtrip, digest);
    assert!(
        serde_json::from_str::<CalibrationReplayDigest>("\"fnv1a64:ABCDEF0123456789\"").is_err()
    );
}

#[test]
fn streaming_counts_each_repository_once_per_pattern_route_and_pending_key() {
    let input = concat!(
        "{\"repo_id\":\"dup/one\",\"name\":\"one\",\"profile_hint\":\"library\",",
        "\"observed_patterns\":[",
        "{\"pattern_id\":\"common.docs.route_present\",\"raw_label\":\"docs-a\",\"count\":2},",
        "{\"pattern_id\":\"common.docs.route_present\",\"raw_label\":\"docs-b\",\"route\":\"docs\",\"count\":3},",
        "{\"raw_label\":\"unknown-x\",\"count\":4},",
        "{\"raw_label\":\"unknown-x\",\"count\":5}]}",
        "\n"
    );
    let run = calibrate_jsonl_reader(Cursor::new(input), metadata()).expect("streaming run");
    let docs = run
        .stats
        .iter()
        .find(|stat| stat.pattern_id == "common.docs.route_present")
        .expect("docs stat");
    let route = run
        .route_requirements
        .iter()
        .find(|requirement| requirement.route == seiri_core::RouteKind::Docs)
        .expect("docs route");
    let pending = run
        .pending_patterns
        .iter()
        .find(|candidate| candidate.raw_label == "unknown-x")
        .expect("pending stat");

    assert_eq!(docs.repositories, 1);
    assert_eq!(docs.observations, 5);
    assert_eq!(route.supporting_repositories, 1);
    assert_eq!(route.observations, 5);
    assert_eq!(pending.observed_repositories, 1);
    assert_eq!(pending.observations, 9);
}

#[test]
fn streaming_limits_fail_closed() {
    let short_limits = StreamingCalibrationLimits::new(16, 8, 8, 8).expect("short limits");
    let line_error = calibrate_jsonl_reader_with_limits(
        Cursor::new("{\"repo_id\":\"long-record\",\"name\":\"long-record\"}\n"),
        metadata(),
        short_limits,
    )
    .expect_err("line limit");
    assert!(matches!(
        line_error,
        seiri_calibration::CalibrationError::StreamingLimitExceeded {
            resource: StreamingLimitKind::LineBytes,
            ..
        }
    ));

    let pattern_limits = StreamingCalibrationLimits::new(4096, 1, 8, 8).expect("pattern limits");
    let two_patterns = concat!(
        "{\"repo_id\":\"patterns/two\",\"name\":\"two\",\"observed_patterns\":[",
        "{\"raw_label\":\"one\"},{\"raw_label\":\"two\"}]}\n"
    );
    let pattern_error =
        calibrate_jsonl_reader_with_limits(Cursor::new(two_patterns), metadata(), pattern_limits)
            .expect_err("pattern limit");
    assert!(matches!(
        pattern_error,
        seiri_calibration::CalibrationError::StreamingLimitExceeded {
            resource: StreamingLimitKind::PatternsPerRecord,
            ..
        }
    ));

    let pending_limits = StreamingCalibrationLimits::new(4096, 8, 1, 8).expect("pending limits");
    let pending_error =
        calibrate_jsonl_reader_with_limits(Cursor::new(two_patterns), metadata(), pending_limits)
            .expect_err("pending limit");
    assert!(matches!(
        pending_error,
        seiri_calibration::CalibrationError::StreamingLimitExceeded {
            resource: StreamingLimitKind::PendingPatterns,
            ..
        }
    ));

    let metadata_limits = StreamingCalibrationLimits::new(4096, 8, 8, 1).expect("metadata limits");
    let metadata_input = concat!(
        "{\"repo_id\":\"metadata/a\",\"name\":\"a\",\"metadata_source\":\"source-a\"}\n",
        "{\"repo_id\":\"metadata/b\",\"name\":\"b\",\"metadata_source\":\"source-b\"}\n"
    );
    let metadata_error = calibrate_jsonl_reader_with_limits(
        Cursor::new(metadata_input),
        metadata(),
        metadata_limits,
    )
    .expect_err("metadata limit");
    assert!(matches!(
        metadata_error,
        seiri_calibration::CalibrationError::StreamingLimitExceeded {
            resource: StreamingLimitKind::MetadataSources,
            ..
        }
    ));

    let utf8_error = calibrate_jsonl_reader(Cursor::new(vec![0xff, b'\n']), metadata())
        .expect_err("invalid UTF-8");
    assert!(matches!(
        utf8_error,
        seiri_calibration::CalibrationError::InvalidUtf8 { line: 1 }
    ));
}

#[test]
fn local_only_streaming_digest_is_redacted_from_reports() {
    let path = fixture("calibration-records.jsonl");
    let run = seiri_report::calibrate_dataset_path(&path).expect("report streaming path");
    assert_eq!(
        run.resource_trace.aggregation_mode,
        CalibrationAggregationMode::StreamingJsonl
    );
    assert_eq!(
        run.resource_trace.record_identity,
        CalibrationRecordIdentity::OneNonemptyJsonlLinePerRepository
    );
    assert!(run.resource_trace.replay_digest.is_some());

    let public = run.redacted_for_public_output();
    assert!(public.resource_trace.replay_digest.is_none());
    let json = seiri_report::calibration_to_json(&run).expect("public calibration JSON");
    assert!(json.contains("\"aggregation_mode\": \"streaming_jsonl\""));
    assert!(json.contains("\"replay_digest\": null"));
    let markdown = seiri_report::calibration_to_markdown(&run);
    assert!(markdown.contains("Aggregation: `StreamingJsonl`"));
    assert!(markdown.contains("Replay digest: `redacted_or_unavailable`"));
}

#[test]
fn large_stream_retains_only_bounded_aggregate_state() {
    const RECORDS: usize = 20_000;
    let path = temporary_jsonl_path();
    {
        let mut writer = BufWriter::new(File::create(&path).expect("create large fixture"));
        for index in 0..RECORDS {
            writeln!(
                writer,
                "{{\"repo_id\":\"large/{index}\",\"name\":\"repo-{index}\",\"metadata_source\":\"synthetic-large\",\"observed_patterns\":[{{\"pattern_id\":\"common.identity.readme_present\",\"raw_label\":\"root_readme\"}}]}}"
            )
            .expect("write record");
        }
        writer.flush().expect("flush large fixture");
    }

    let run = calibrate_jsonl_reader(
        BufReader::new(File::open(&path).expect("open large fixture")),
        StreamingCalibrationMetadata::new("large-stream", "large-stream", "unknown"),
    )
    .expect("large streaming calibration");
    fs::remove_file(&path).expect("remove large fixture");

    assert_eq!(run.summary.records, RECORDS);
    assert_eq!(run.resource_trace.records_seen, RECORDS);
    assert_eq!(run.resource_trace.retained_records, 0);
    assert_eq!(run.resource_trace.retained_repository_id_entries, 0);
    assert_eq!(run.resource_trace.per_pattern_repository_sets, 0);
    assert_eq!(run.resource_trace.pending_pattern_slots, 0);
    assert_eq!(run.resource_trace.metadata_source_slots, 1);
    assert_eq!(run.resource_trace.max_patterns_per_record, 1);
    assert!(run.resource_trace.replay_digest.is_some());
    let identity = run
        .stats
        .iter()
        .find(|stat| stat.pattern_id == "common.identity.readme_present")
        .expect("identity stat");
    assert_eq!(identity.repositories, RECORDS);
}

#[test]
fn calibration_run_requires_resource_trace() {
    let dataset =
        seiri_calibration::load_dataset(fixture("calibration-dataset.json")).expect("dataset");
    let run = seiri_calibration::calibrate_dataset(&dataset).expect("calibrate dataset");
    let mut value = serde_json::to_value(run).expect("run value");
    value
        .as_object_mut()
        .expect("run object")
        .remove("resource_trace");

    assert!(serde_json::from_value::<seiri_core::CalibrationRun>(value).is_err());
}

fn temporary_jsonl_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "reposeiri-streaming-calibration-{}-{nonce}.jsonl",
        std::process::id()
    ))
}
