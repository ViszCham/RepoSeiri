use seiri_core::{
    ContractManifest, ErrorClass, ErrorEnvelope, ANALYSIS_SCHEMA_VERSION, CODEX_SCHEMA_VERSION,
    COMPLETION_SCHEMA_VERSION, ERROR_SCHEMA_VERSION, PATCH_PLAN_SCHEMA_VERSION,
};
use std::fs;
use std::path::PathBuf;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn public_contract_is_v2_only() {
    assert_eq!(ANALYSIS_SCHEMA_VERSION, "seiri.analysis.v2");
    assert_eq!(PATCH_PLAN_SCHEMA_VERSION, "seiri.patch-plan.v2");
    assert_eq!(CODEX_SCHEMA_VERSION, "seiri.codex.v2");
    assert_eq!(ERROR_SCHEMA_VERSION, "seiri.error.v1");
    assert_eq!(COMPLETION_SCHEMA_VERSION, "seiri.completion.v3");

    let manifest = ContractManifest::current("1.0.0");
    let json = serde_json::to_string(&manifest).expect("contract JSON");
    assert!(!json.contains("analysis.v1"));
    assert!(!json.contains("patch-plan.v1"));
    assert!(!json.contains("codex.v1"));
    assert!(manifest.compatibility.starts_with("v2-only"));
    assert_eq!(
        manifest.semantic_revisions.claim_projection,
        seiri_core::CLAIM_SEMANTIC_REVISION
    );
    assert_eq!(
        manifest.semantic_revisions.patch_planner,
        "seiri.patch-planner.v4"
    );
    assert_eq!(
        manifest.semantic_revisions.entries().len(),
        seiri_core::SemanticRevisionKey::ALL.len()
    );
    manifest.validate_current().expect("current contract");
}

#[test]
fn contract_rejects_missing_unknown_and_unsupported_semantics() {
    let manifest = ContractManifest::current("1.0.0");
    let mut missing = serde_json::to_value(&manifest).expect("manifest value");
    missing["semantic_revisions"]
        .as_object_mut()
        .expect("semantic object")
        .remove("markdown_parser");
    assert!(serde_json::from_value::<ContractManifest>(missing).is_err());

    let mut unknown = serde_json::to_value(&manifest).expect("manifest value");
    unknown["semantic_revisions"]["future_revision"] = serde_json::json!("future.v1");
    assert!(serde_json::from_value::<ContractManifest>(unknown).is_err());

    let mut unsupported = manifest;
    unsupported.semantic_revisions.delta = "seiri.audit-delta-semantics.v999".to_string();
    assert!(matches!(
        unsupported.validate_current(),
        Err(seiri_core::ContractValidationError::SemanticRevision(
            seiri_core::SemanticRevisionKey::Delta
        ))
    ));
}

#[test]
fn active_schema_snapshots_match_owned_constants() {
    let cases = [
        ("seiri.analysis.v2.json", ANALYSIS_SCHEMA_VERSION),
        ("seiri.patch-plan.v2.json", PATCH_PLAN_SCHEMA_VERSION),
        ("seiri.codex.v2.json", CODEX_SCHEMA_VERSION),
        ("seiri.error.v1.json", ERROR_SCHEMA_VERSION),
        ("seiri.completion.v3.json", COMPLETION_SCHEMA_VERSION),
    ];
    for (file, expected) in cases {
        let body = fs::read_to_string(repository_root().join("schemas").join(file))
            .expect("schema snapshot");
        let value: serde_json::Value = serde_json::from_str(&body).expect("valid schema JSON");
        assert_eq!(value["schema_version"], expected);
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(value["type"], "object");
        assert_eq!(value["properties"]["schema_version"]["const"], expected);
        assert_ne!(value["compatibility"], "v1-compatible");
        assert_eq!(value["additionalProperties"], false);
        for property in value["properties"]
            .as_object()
            .expect("schema properties")
            .values()
        {
            if property["type"] == "array" {
                assert!(
                    property.get("items").is_some(),
                    "top-level arrays must freeze their item schema"
                );
            }
        }
    }
    for (file, expected) in [
        (
            "seiri.portable-audit.v2.json",
            seiri_core::PORTABLE_AUDIT_SCHEMA_VERSION,
        ),
        (
            "seiri.audit-delta.v2.json",
            seiri_core::AUDIT_DELTA_SCHEMA_VERSION,
        ),
        ("seiri.calibration.v2.json", "seiri.calibration.v2"),
        (
            "seiri.local-calibration-priors.v2.json",
            seiri_calibration::LOCAL_PRIOR_SCHEMA_VERSION,
        ),
        (
            "seiri.executable-pattern-pack.v2.json",
            seiri_patterns::EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION,
        ),
    ] {
        let body = fs::read_to_string(repository_root().join("schemas").join(file))
            .expect("extension schema snapshot");
        let value: serde_json::Value = serde_json::from_str(&body).expect("valid schema JSON");
        assert_eq!(value["schema_version"], expected);
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(value["properties"]["schema_version"]["const"], expected);
        assert_eq!(value["compatibility"], "v2-only");
        assert_eq!(value["additionalProperties"], false);
    }
    for (file, expected) in [
        (
            "seiri.calibration-corpus.v1.json",
            seiri_report::HOLDOUT_CORPUS_SCHEMA_VERSION,
        ),
        (
            "seiri.calibration-holdout.v1.json",
            seiri_report::HOLDOUT_REPORT_SCHEMA_VERSION,
        ),
    ] {
        let body = fs::read_to_string(repository_root().join("schemas").join(file))
            .expect("holdout schema snapshot");
        let value: serde_json::Value = serde_json::from_str(&body).expect("valid schema JSON");
        assert_eq!(value["schema_version"], expected);
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(value["properties"]["schema_version"]["const"], expected);
        assert_eq!(value["compatibility"], "v1-only");
        assert_eq!(value["additionalProperties"], false);
    }
}

#[test]
fn completion_schema_types_process_failures_and_host_receipts() {
    let body = fs::read_to_string(repository_root().join("schemas/seiri.completion.v3.json"))
        .expect("completion schema");
    let schema: serde_json::Value = serde_json::from_str(&body).expect("completion schema JSON");
    let failures = schema["$defs"]["check"]["properties"]["failure_class"]["enum"]
        .as_array()
        .expect("failure classes");
    for class in [
        "missing_executable",
        "could_not_start",
        "environment_blocked",
        "timed_out",
        "output_limit_exceeded",
        "non_zero_exit",
        "io",
    ] {
        assert!(
            failures.iter().any(|value| value == class),
            "completion schema omitted {class}"
        );
    }
    let host_required = schema["$defs"]["host"]["required"]
        .as_array()
        .expect("host required fields");
    for field in [
        "source_digest",
        "cargo_lock_digest",
        "binary_digest",
        "command_set",
    ] {
        assert!(
            host_required.iter().any(|value| value == field),
            "host receipt omitted {field}"
        );
    }
    let states = schema["properties"]["state"]["enum"]
        .as_array()
        .expect("completion states");
    assert!(states
        .iter()
        .any(|state| state == "implemented_with_blocked_evidence"));
    for field in ["calibration", "claims", "evidence_complete"] {
        assert!(
            schema["required"]
                .as_array()
                .expect("completion required fields")
                .iter()
                .any(|value| value == field),
            "completion record omitted {field}"
        );
    }
    assert_eq!(schema["properties"]["claims"]["minItems"], 5);
    assert_eq!(schema["properties"]["claims"]["maxItems"], 5);
}

#[test]
fn typed_error_has_stable_exit_classes() {
    let envelope = ErrorEnvelope::new(ErrorClass::Contract, "schema_mismatch", "mismatch");
    assert_eq!(envelope.schema_version, ERROR_SCHEMA_VERSION);
    assert_eq!(envelope.class.exit_code(), 5);
    assert_eq!(ErrorClass::InvalidInput.exit_code(), 3);
    assert_eq!(ErrorClass::Io.exit_code(), 4);
    assert_eq!(ErrorClass::Internal.exit_code(), 70);
}
