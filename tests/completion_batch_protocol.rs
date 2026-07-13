use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative)).expect("read protocol fixture")
}

fn template() -> Value {
    serde_json::from_str(&read("docs/design/rcbp-v1-template.json"))
        .expect("RCBP-v1 template must be valid JSON")
}

#[test]
fn rcbp_template_freezes_block_order_and_single_slice_execution() {
    let value = template();
    assert_eq!(value["schema_version"], "reposeiri.completion-batch.v1");
    assert_eq!(value["protocol"], "RCBP-v1");

    let blocks = value["blocks"].as_array().expect("blocks must be an array");
    let ids = blocks
        .iter()
        .map(|block| block["id"].as_str().expect("block id"))
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        ["CF0", "CF1", "CF2", "CF3", "CF4", "CF5", "CF6", "CF7"]
    );

    for adjacent in blocks.windows(2) {
        let previous = adjacent[0]["id"].as_str().expect("previous block id");
        let dependencies = adjacent[1]["depends_on"]
            .as_array()
            .expect("dependencies must be an array");
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0], previous);
    }

    assert_eq!(value["execution"]["max_in_progress_slices"], 1);
    assert_eq!(value["execution"]["intermediate_commits"], false);
    assert_eq!(value["execution"]["blocking_checks_may_be_skipped"], false);
}

#[test]
fn rcbp_template_keeps_operational_authorities_separate() {
    let value = template();
    let authority = &value["authority_defaults"];
    assert_eq!(authority["mutation"], true);
    assert_eq!(authority["tests"], true);
    assert_eq!(authority["execution_ledger"], true);

    for denied in [
        "commit",
        "push",
        "merge",
        "release",
        "plugin_install",
        "restart",
        "visibility",
    ] {
        assert_eq!(authority[denied], false, "{denied} must default to false");
    }

    assert_eq!(
        value["final_states"],
        serde_json::json!(["ready_for_git", "incomplete"])
    );
}

#[test]
fn rcbp_public_template_retains_privacy_and_ignored_ledger_boundaries() {
    let body = read("docs/design/rcbp-v1-template.json");
    let value: Value = serde_json::from_str(&body).expect("valid template");
    assert_eq!(
        value["execution"]["ledger_path"],
        "target/rcbp/<execution-id>/state.json"
    );

    for denied in [
        "store_source_bodies",
        "store_diff_bodies",
        "store_private_analysis",
        "store_private_calibration_values",
        "store_credentials",
    ] {
        assert_eq!(value["privacy"][denied], false, "{denied} must be false");
    }

    for local_marker in ["Downloads", "USERPROFILE", "C:\\Users", "/home/"] {
        assert!(
            !body.contains(local_marker),
            "public template contains local marker {local_marker}"
        );
    }
}

#[test]
fn agents_and_design_indexes_expose_the_rcbp_trigger_and_authority() {
    let agents = read("AGENTS.md");
    let design_index = read("docs/design/README.md");
    let docs_index = read("docs/README.md");

    for required in [
        "RCBP-v1でCF0-CF7を一括実装してください",
        "MutationAuthority",
        "TestAuthority",
        "ready_for_git",
        "incomplete",
    ] {
        assert!(agents.contains(required), "AGENTS.md is missing {required}");
    }

    for required in [
        "roadmap-v6-completion.md",
        "completion-batch-protocol.md",
        "rcbp-v1-template.json",
    ] {
        assert!(
            design_index.contains(required),
            "design index is missing {required}"
        );
    }

    for required in ["roadmap-v6-completion.md", "completion-batch-protocol.md"] {
        assert!(
            docs_index.contains(required),
            "docs index is missing {required}"
        );
    }
}
