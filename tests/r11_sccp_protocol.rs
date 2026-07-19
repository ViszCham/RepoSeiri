use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn template() -> Value {
    let body = fs::read_to_string(root().join("docs/design/r11-sccp-v1-template.json"))
        .expect("read R11-SCCP template");
    serde_json::from_str(&body).expect("valid R11-SCCP template")
}

#[test]
fn r11_sccp_runs_k0_through_k12_in_dependency_order() {
    let value = template();
    assert_eq!(value["schema_version"], "reposeiri.r11-sccp.v1");
    assert_eq!(value["protocol"], "R11-SCCP-v1");
    let units = value["units"].as_array().expect("units");
    assert_eq!(units.len(), 13);
    for (index, unit) in units.iter().enumerate() {
        assert_eq!(unit["id"], format!("K{index}"));
        let dependencies = unit["depends_on"].as_array().expect("dependencies");
        if index == 0 {
            assert!(dependencies.is_empty());
        } else {
            assert_eq!(dependencies, &[Value::String(format!("K{}", index - 1))]);
        }
    }
}

#[test]
fn r11_sccp_is_noninteractive_without_operational_authority() {
    let value = template();
    assert_eq!(value["execution"]["mode"], "continuous_noninteractive");
    assert_eq!(value["execution"]["stop_on_first_failure"], false);
    assert_eq!(value["execution"]["blocking_checks_may_be_skipped"], false);
    assert_eq!(value["execution"]["blocked_checks_may_be_promoted"], false);
    assert_eq!(value["authority_defaults"]["mutation"], true);
    assert_eq!(value["authority_defaults"]["verification"], true);
    for denied in [
        "commit",
        "push",
        "merge",
        "release",
        "publication",
        "visibility",
        "plugin_install",
        "restart",
    ] {
        assert_eq!(value["authority_defaults"][denied], false);
    }
}

#[test]
fn r11_sccp_freezes_semantic_delta_and_privacy_boundaries() {
    let value = template();
    assert_eq!(
        value["semantic_delta_allowlist"]
            .as_array()
            .expect("allowlist")
            .len(),
        4
    );
    for field in [
        "store_source_bodies",
        "store_diff_bodies",
        "store_private_analysis_identity",
        "store_private_analysis_body",
        "store_private_calibration_values",
        "store_private_digest",
        "store_host_absolute_paths",
        "store_credentials",
    ] {
        assert_eq!(value["privacy"][field], false);
    }
}

#[test]
fn r11_documents_are_japanese_first_and_english_second() {
    for path in [
        "docs/design/roadmap-v11-semantic-compression.md",
        "docs/design/r11-sccp-v1-protocol.md",
    ] {
        let body = fs::read_to_string(root().join(path)).expect("read R11 document");
        let japanese = body.find("## 日本語").expect("Japanese section");
        let english = body.find("## English").expect("English section");
        assert!(japanese < english);
        for term in ["K0", "K12", "ready_for_git"] {
            assert!(body[japanese..english].contains(term));
            assert!(body[english..].contains(term));
        }
    }
}
