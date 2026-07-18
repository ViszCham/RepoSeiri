use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn template() -> Value {
    let body = fs::read_to_string(root().join("docs/design/r10-sip-v1-template.json"))
        .expect("read R10-SIP template");
    serde_json::from_str(&body).expect("valid R10-SIP template")
}

#[test]
fn r10_sip_runs_c0_through_c8_in_dependency_order() {
    let value = template();
    assert_eq!(value["schema_version"], "reposeiri.r10-sip.v1");
    assert_eq!(value["protocol"], "R10-SIP-v1");

    let units = value["units"].as_array().expect("units");
    assert_eq!(units.len(), 9);
    for (index, unit) in units.iter().enumerate() {
        assert_eq!(unit["id"], format!("C{index}"));
        let dependencies = unit["depends_on"].as_array().expect("dependencies");
        if index == 0 {
            assert!(dependencies.is_empty());
        } else {
            assert_eq!(dependencies, &[Value::String(format!("C{}", index - 1))]);
        }
    }
}

#[test]
fn r10_sip_is_noninteractive_but_never_promotes_blocked_evidence() {
    let value = template();
    let execution = &value["execution"];
    assert_eq!(execution["mode"], "continuous_noninteractive");
    assert_eq!(execution["midrun_user_prompt"], false);
    assert_eq!(execution["stop_on_first_failure"], false);
    assert_eq!(execution["continue_independent_after_block"], true);
    assert_eq!(execution["blocking_checks_may_be_skipped"], false);
    assert_eq!(execution["blocked_checks_may_be_promoted"], false);

    assert!(execution["local_repair_limit_per_slice"]
        .as_u64()
        .is_some_and(|limit| limit > 0));
    assert!(execution["owner_backflow_limit"]
        .as_u64()
        .is_some_and(|limit| limit > 0));
    assert!(execution["global_repair_limit"]
        .as_u64()
        .is_some_and(|limit| limit > 0));

    assert_eq!(
        value["failure_policies"]["environment_blocked"],
        "record_and_continue_independent"
    );
    assert_eq!(
        value["failure_policies"]["privacy_boundary"],
        "remove_protocol_leak_preserve_user_source_run_privacy_regression_and_continue"
    );
}

#[test]
fn r10_sip_keeps_mutation_separate_from_git_release_and_install_authority() {
    let value = template();
    let authority = &value["authority_defaults"];
    assert_eq!(authority["mutation"], true);
    assert_eq!(authority["verification"], true);

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
        assert_eq!(authority[denied], false, "{denied} must remain denied");
    }
}

#[test]
fn r10_sip_completion_requires_coverage_source_binding_and_host_evidence() {
    let value = template();
    let completion = &value["completion"];
    assert_eq!(completion["finding_zero_is_sufficient"], false);
    for required in [
        "requires_complete_primary_coverage",
        "requires_zero_unacknowledged_unknown",
        "requires_zero_budget_exhaustion",
        "requires_complete_conflict_coverage",
        "requires_same_source_binding",
        "requires_stable_identity_properties",
        "requires_contract_revision_parity",
        "requires_extension_positive_negative_privacy_tests",
        "requires_markdown_and_bilingual_corpora",
        "requires_product_surface_parity",
        "requires_local_verification",
        "requires_windows_and_linux_receipts_for_evidence_complete",
        "requires_holdout_uncertainty_for_calibrated",
    ] {
        assert_eq!(completion[required], true, "{required} must be required");
    }
}

#[test]
fn r10_sip_documents_are_japanese_first_and_english_equivalent_surfaces() {
    for path in [
        "docs/design/roadmap-v10-closure-and-product-integrity.md",
        "docs/design/r10-sip-v1-protocol.md",
    ] {
        let body = fs::read_to_string(root().join(path)).expect("read design document");
        let japanese = body.find("## 日本語").expect("Japanese section");
        let english = body.find("## English").expect("English section");
        assert!(japanese < english);

        let japanese_body = &body[japanese..english];
        let english_body = &body[english..];
        for term in ["C0", "C8", "ready_for_git", "EVIDENCE_COMPLETE"] {
            assert!(japanese_body.contains(term), "{path}: Japanese {term}");
            assert!(english_body.contains(term), "{path}: English {term}");
        }
    }
}

#[test]
fn design_indexes_route_to_the_current_r10_contract() {
    for path in ["docs/design/README.md", "docs/README.md"] {
        let body = fs::read_to_string(root().join(path)).expect("read documentation index");
        assert!(
            body.contains("roadmap-v10-closure-and-product-integrity.md"),
            "{path}: Roadmap v10 route"
        );
        assert!(
            body.contains("r10-sip-v1-protocol.md"),
            "{path}: R10-SIP-v1 route"
        );
    }
}
