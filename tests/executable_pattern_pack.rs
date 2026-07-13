use seiri_core::{
    CalibrationKey, CalibrationLookup, CalibrationProvider, PatternGroup, ProfileKind, RouteKind,
};
use seiri_patterns::{
    evaluate_adoption_gate, load_executable_pattern_pack, AdoptionBlocker, AdoptionGateDecision,
    PatternAdoptionReview, PatternFixtureKind, PatternPackLoadError,
    EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn loads_and_executes_all_group_fixture_classes_without_process_or_network() {
    let fixture = BlockZFixture::new();
    let pack = load_executable_pattern_pack(&fixture.pack_path).expect("load executable pack");
    assert_eq!(pack.fixtures().len(), PatternGroup::ALL.len() * 5);
    assert_eq!(
        pack.fingerprint(),
        load_executable_pattern_pack(&fixture.pack_path)
            .expect("reload executable pack")
            .fingerprint()
    );

    let report = seiri_report::run_executable_pattern_pack(&pack);
    assert!(report.all_passed(), "{:#?}", report.results);
    assert_eq!(report.subprocesses_started, 0);
    assert_eq!(report.network_requests_started, 0);
    for group in PatternGroup::ALL {
        for kind in PatternFixtureKind::ALL {
            let prefix = format!(
                "fixture.{}.{}",
                group.code().to_ascii_lowercase(),
                kind.slug()
            );
            assert!(report.results.iter().any(|result| {
                result.fixture_id == prefix
                    && result.status == seiri_patterns::FixtureExecutionStatus::Passed
            }));
        }
    }
}

#[test]
fn loader_rejects_unbounded_paths_invalid_predicates_and_partial_absence() {
    let fixture = BlockZFixture::new();

    let mut path_escape = fixture.pack_value.clone();
    path_escape["fixtures"][0]["root"] = json!("../outside");
    let path = fixture.write_variant("path-escape.json", &path_escape);
    assert!(matches!(
        load_executable_pattern_pack(path),
        Err(PatternPackLoadError::InvalidFixturePath)
    ));

    let mut invalid_predicate = fixture.pack_value.clone();
    invalid_predicate["definitions"][0]["predicate"]["instructions"] =
        json!([{ "op": "push_atom", "data": 1 }]);
    let path = fixture.write_variant("invalid-predicate.json", &invalid_predicate);
    assert!(matches!(
        load_executable_pattern_pack(path),
        Err(PatternPackLoadError::InvalidPredicate(_))
    ));

    let mut too_many_expectations = fixture.pack_value.clone();
    let expectation = too_many_expectations["fixtures"][0]["expectations"][0].clone();
    too_many_expectations["fixtures"][0]["expectations"] = Value::Array(vec![
        expectation;
        seiri_patterns::MAX_FIXTURE_EXPECTATIONS
            + 1
    ]);
    let path = fixture.write_variant("too-many-expectations.json", &too_many_expectations);
    assert!(matches!(
        load_executable_pattern_pack(path),
        Err(PatternPackLoadError::InvalidExpectationCount)
    ));

    let mut partial_absence = fixture.pack_value.clone();
    let partial_index = partial_absence["fixtures"]
        .as_array()
        .expect("fixture array")
        .iter()
        .position(|fixture| fixture["kind"] == "partial")
        .expect("partial fixture");
    let pattern_id = partial_absence["definitions"][0]["id"].clone();
    partial_absence["fixtures"][partial_index]["expectations"] = json!([{
        "kind": "pattern",
        "pattern": pattern_id,
        "outcome": "missing",
        "evidence": { "kind": "any" }
    }]);
    let path = fixture.write_variant("partial-absence.json", &partial_absence);
    assert!(matches!(
        load_executable_pattern_pack(path),
        Err(PatternPackLoadError::PartialExpectsAbsence)
    ));

    let mut oversized = fixture.pack_value.clone();
    oversized["fixtures"][0]["scan_budget"]["max_file_bytes"] = json!(1);
    let path = fixture.write_variant("oversized-fixture.json", &oversized);
    assert!(matches!(
        load_executable_pattern_pack(path),
        Err(PatternPackLoadError::FixtureFileTooLarge)
    ));
}

#[test]
fn compares_evidence_ids_and_requires_review_before_adoption() {
    let fixture = BlockZFixture::new();
    let mut wrong_evidence = fixture.pack_value.clone();
    wrong_evidence["fixtures"][0]["expectations"][0]["evidence"] =
        json!({ "kind": "exact", "data": ["evrec-9999"] });
    let path = fixture.write_variant("wrong-evidence.json", &wrong_evidence);
    let pack = load_executable_pattern_pack(path).expect("load evidence comparison pack");
    let report = seiri_report::run_executable_pattern_pack(&pack);
    assert!(!report.all_passed());
    assert!(report.results[0].expectations[0]
        .actual
        .to_string_for_test()
        .contains("evrec-"));

    let blocked =
        evaluate_adoption_gate(&pack, &report, None, EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION);
    assert!(matches!(
        blocked,
        AdoptionGateDecision::Blocked(ref blockers)
            if blockers.contains(&AdoptionBlocker::FixtureFailure)
                && blockers.contains(&AdoptionBlocker::MissingReview)
    ));

    let valid_pack =
        load_executable_pattern_pack(&fixture.pack_path).expect("load valid executable pack");
    let valid_report = seiri_report::run_executable_pattern_pack(&valid_pack);
    let review = PatternAdoptionReview {
        review_id: "review-private-overlay".to_string(),
        reviewer: "maintainer".to_string(),
        reviewed_pack_fingerprint: valid_pack.fingerprint().to_string(),
    };
    assert_eq!(
        evaluate_adoption_gate(
            &valid_pack,
            &valid_report,
            Some(&review),
            EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION,
        ),
        AdoptionGateDecision::EligibleForMaintainerAdoption
    );
    assert!(valid_pack
        .definitions()
        .iter()
        .all(|definition| definition.adoption_stage
            == seiri_patterns::PatternAdoptionStage::Candidate));
}

#[test]
fn private_overlay_applies_locally_without_public_prior_values() {
    let fixture = BlockZFixture::new();
    let pack = load_executable_pattern_pack(&fixture.pack_path).expect("load executable pack");
    let private_prior = json!({
        "schema_version": seiri_calibration::LOCAL_PRIOR_SCHEMA_VERSION,
        "registry_fingerprint": pack.fingerprint(),
        "_private_note": "C:/PRIVATE_OVERLAY_SENTINEL",
        "priors": [{
            "key": { "kind": "route_gap", "route": "docs" },
            "observed": 7,
            "sample_size": 10,
            "rank_weight_x100": 91
        }]
    });
    let prior_path = fixture.write_variant("private-priors.json", &private_prior);
    let overlay =
        seiri_calibration::load_private_calibration_overlay(&fixture.pack_path, &prior_path)
            .expect("load private overlay");
    assert_eq!(overlay.registry_fingerprint(), pack.fingerprint());
    let metadata = overlay.metadata();
    assert_eq!(
        metadata.schema_version,
        seiri_calibration::PRIVATE_OVERLAY_METADATA_SCHEMA_VERSION
    );
    assert_eq!(metadata.visibility, "local_only");
    assert!(metadata.source_path_redacted);
    assert!(metadata.source_body_redacted);
    assert!(metadata.exact_priors_redacted);
    assert_eq!(metadata.resource_trace.prior_count, 1);
    let metadata_json = serde_json::to_string(&metadata).expect("metadata JSON");
    assert!(!metadata_json.contains("PRIVATE_OVERLAY_SENTINEL"));
    assert!(!metadata_json.contains(prior_path.to_string_lossy().as_ref()));
    assert!(matches!(
        overlay.prior(&CalibrationKey::RouteGap(RouteKind::Docs)),
        CalibrationLookup::Available(_)
    ));

    let snapshot = seiri_report::audit_repository_with_calibration_provider(
        fixture.root.join("positive"),
        ProfileKind::Common,
        &overlay,
    )
    .expect("audit with private overlay");
    let json = seiri_report::to_json(&snapshot).expect("public report JSON");
    let markdown = seiri_report::to_markdown(&snapshot);
    for output in [&json, &markdown] {
        assert!(!output.contains("PRIVATE_OVERLAY_SENTINEL"));
        assert!(!output.contains("sample_size"));
        assert!(!output.contains("rank_weight_x100"));
        assert!(!output.contains("0.700"));
    }
}

#[test]
fn removed_v1_private_overlay_wire_is_rejected() {
    let fixture = BlockZFixture::new();
    let pack = load_executable_pattern_pack(&fixture.pack_path).expect("load executable pack");
    let removed = json!({
        "schema_version": "seiri.local-calibration-priors.v1",
        "registry_fingerprint": pack.fingerprint(),
        "priors": []
    });
    let path = fixture.write_variant("removed-v1-priors.json", &removed);
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider_for_registry(path, pack.fingerprint()),
        Err(seiri_calibration::LocalPriorLoadError::UnsupportedSchema)
    ));
}

trait ActualTestString {
    fn to_string_for_test(&self) -> String;
}

impl ActualTestString for seiri_patterns::FixtureExpectationActual {
    fn to_string_for_test(&self) -> String {
        serde_json::to_string(self).expect("actual JSON")
    }
}

struct BlockZFixture {
    root: PathBuf,
    pack_path: PathBuf,
    pack_value: Value,
}

impl BlockZFixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "reposeiri-pattern-pack-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("positive")).expect("positive root");
        fs::write(root.join("positive/README.md"), "# Positive\n").expect("positive file");
        fs::create_dir_all(root.join("negative")).expect("negative root");
        fs::create_dir_all(root.join("ambiguous")).expect("ambiguous root");
        fs::write(root.join("ambiguous/README.md"), "# Ambiguous\n").expect("ambiguous file");
        fs::create_dir_all(root.join("partial")).expect("partial root");
        fs::write(root.join("partial/a.md"), "# A\n").expect("partial a");
        fs::write(root.join("partial/b.md"), "# B\n").expect("partial b");
        fs::create_dir_all(root.join("malformed/.github")).expect("malformed root");
        fs::write(root.join("malformed/.github/CODEOWNERS"), "/src/\n")
            .expect("malformed CODEOWNERS");

        let pack_value = executable_pack_value();
        let pack_path = root.join("pack.json");
        write_json(&pack_path, &pack_value);
        Self {
            root,
            pack_path,
            pack_value,
        }
    }

    fn write_variant(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.root.join(name);
        write_json(&path, value);
        path
    }
}

impl Drop for BlockZFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn executable_pack_value() -> Value {
    let definitions = PatternGroup::ALL
        .iter()
        .map(|group| {
            json!({
                "id": format!("data.{}", group.code().to_ascii_lowercase()),
                "group": group.code(),
                "predicate": {
                    "atoms": [{
                        "kind": "evidence",
                        "data": { "kind": "file_present" }
                    }],
                    "instructions": [{ "op": "push_atom", "data": 0 }]
                },
                "boundaries": [
                    "not_automatic_policy_adoption",
                    "not_automatic_weight_adoption"
                ],
                "adoption_stage": "candidate"
            })
        })
        .collect::<Vec<_>>();

    let mut fixtures = Vec::new();
    for group in PatternGroup::ALL {
        let pattern = format!("data.{}", group.code().to_ascii_lowercase());
        for kind in PatternFixtureKind::ALL {
            let (root, expectations, max_entries) = match kind {
                PatternFixtureKind::Positive => (
                    "positive",
                    json!([{
                        "kind": "pattern",
                        "pattern": pattern,
                        "outcome": "present",
                        "evidence": { "kind": "at_least", "data": 1 }
                    }]),
                    100,
                ),
                PatternFixtureKind::Negative => (
                    "negative",
                    json!([{
                        "kind": "pattern",
                        "pattern": pattern,
                        "outcome": "missing",
                        "evidence": { "kind": "exact", "data": [] }
                    }]),
                    100,
                ),
                PatternFixtureKind::Ambiguous => (
                    "ambiguous",
                    json!([{
                        "kind": "coverage",
                        "scope": { "kind": "repository_files" },
                        "status": { "kind": "complete" }
                    }]),
                    100,
                ),
                PatternFixtureKind::Partial => (
                    "partial",
                    json!([{
                        "kind": "coverage",
                        "scope": { "kind": "repository_files" },
                        "status": { "kind": "partial", "reason": "limit_exceeded" }
                    }]),
                    1,
                ),
                PatternFixtureKind::Malformed => (
                    "malformed",
                    json!([{ "kind": "diagnostic", "minimum": 1 }]),
                    100,
                ),
            };
            fixtures.push(json!({
                "id": format!(
                    "fixture.{}.{}",
                    group.code().to_ascii_lowercase(),
                    kind.slug()
                ),
                "kind": kind.slug(),
                "group": group.code(),
                "root": root,
                "expectations": expectations,
                "scan_budget": {
                    "max_depth": 8,
                    "max_entries": max_entries,
                    "max_file_bytes": 1048576,
                    "max_total_bytes": 4194304
                }
            }));
        }
    }
    json!({
        "schema_version": EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION,
        "id": "executable-fixture-pack",
        "version": "1",
        "definitions": definitions,
        "fixtures": fixtures
    })
}

fn write_json(path: &Path, value: &Value) {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize JSON fixture"),
    )
    .expect("write JSON fixture");
}
