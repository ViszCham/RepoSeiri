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
    assert_eq!(COMPLETION_SCHEMA_VERSION, "seiri.completion.v2");

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
        "seiri.patch-planner.v3"
    );
}

#[test]
fn active_schema_snapshots_match_owned_constants() {
    let cases = [
        ("seiri.analysis.v2.json", ANALYSIS_SCHEMA_VERSION),
        ("seiri.patch-plan.v2.json", PATCH_PLAN_SCHEMA_VERSION),
        ("seiri.codex.v2.json", CODEX_SCHEMA_VERSION),
        ("seiri.error.v1.json", ERROR_SCHEMA_VERSION),
        ("seiri.completion.v2.json", COMPLETION_SCHEMA_VERSION),
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
    }
    for (file, expected) in [
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
    }
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
