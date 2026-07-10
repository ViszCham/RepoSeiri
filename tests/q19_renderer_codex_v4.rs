use seiri_core::{
    CodexCommand, CodexQueryData, CodexQueryKind, ProfileKind, RepoSnapshot,
    CODEX_KERNEL_SCHEMA_VERSION, CODEX_LINTER_CONTEXT_SCHEMA_VERSION, CODEX_NATIVE_SCHEMA_VERSION,
    CODEX_QUERY_SCHEMA_VERSION,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn build_kernel(name: &str, profile: ProfileKind) -> seiri_codex::CodexReviewKernel {
    let root = fixture(name);
    let snapshot = seiri_report::audit_repository_with_profile(&root, profile).expect("audit");
    let plan = seiri_planner::plan_compatibility_safe_patches(&snapshot);
    let linter =
        seiri_report::lint_wording_repository_with_profile(&root, profile).expect("wording lint");
    seiri_codex::build_review_kernel(&snapshot, &plan, Some(&linter))
}

#[test]
fn q19_kernel_projects_stable_v1_and_native_v2_views() {
    let kernel = build_kernel("safe-plan-repo", ProfileKind::Common);
    let compatibility = kernel.compatibility_v1();
    let native = kernel.native_v2();

    assert_eq!(compatibility.schema_version, seiri_core::SCHEMA_VERSION);
    assert_eq!(native.schema_version, CODEX_NATIVE_SCHEMA_VERSION);
    assert_eq!(native.kernel_schema_version, CODEX_KERNEL_SCHEMA_VERSION);
    assert_eq!(compatibility.repo_root, native.repo_root);
    assert_eq!(compatibility.profile, native.profile);
    assert_eq!(
        compatibility.audit.entries_scanned,
        native.audit.entries_scanned
    );
    assert_eq!(
        compatibility.audit.evidence_kernel_facts,
        native.evidence_kernel.facts().len()
    );
    assert_eq!(
        compatibility.audit.route_assessments,
        native.route_assessments.len()
    );
    assert_eq!(compatibility.audit.content_claims, native.claims.len());
    assert_eq!(compatibility.plan, native.plan.summary);
    assert_eq!(native.plan.operations.len(), 1);
    assert_eq!(
        native.plan.operations[0].proposal.schema_version,
        seiri_core::PATCH_PROPOSAL_SCHEMA_VERSION
    );

    let compatibility_json = seiri_report::codex_to_json(&compatibility).expect("v1 JSON");
    let native_json = seiri_report::codex_native_to_json(&native).expect("v2 JSON");
    let native_value: serde_json::Value = serde_json::from_str(&native_json).expect("native JSON");
    assert!(compatibility_json.contains("\"user_actions\""));
    assert!(compatibility_json.contains("\"pr_draft\""));
    assert!(native_value.get("evidence_kernel").is_some());
    assert!(native_value.get("route_assessments").is_some());
    assert!(native_value.get("actions").is_some());
    assert!(native_value["audit"].get("evidence_items").is_none());
    assert!(native_value["audit"]
        .get("evidence_ledger_records")
        .is_none());
    assert!(native_value["audit"].get("route_states").is_none());
    for compatibility_only in [
        "readme",
        "evidence",
        "evidence_ledger",
        "route_states",
        "safe_operations",
        "blocked_items",
        "user_actions",
        "pr_draft",
    ] {
        assert!(
            native_value.get(compatibility_only).is_none(),
            "native v2 leaked compatibility field `{compatibility_only}`"
        );
    }
    assert!(native_json.contains("\"program\": \"cargo\""));
    assert!(native_json.contains("\"args\""));
}

#[test]
fn q19_argv_commands_preserve_arguments_and_validate_wire_input() {
    let path = "C:\\repo with 'quote'; $(Get-Item secret)";
    let command = CodexCommand::new("cargo", ["run", "--path", path]).expect("valid argv");

    assert_eq!(command.program(), "cargo");
    assert_eq!(command.args(), ["run", "--path", path]);
    assert_eq!(
        command.render_powershell(),
        "cargo run --path 'C:\\repo with ''quote''; $(Get-Item secret)'"
    );
    let value = serde_json::to_value(&command).expect("command JSON");
    assert_eq!(value["args"][2], path);
    assert!(serde_json::from_value::<CodexCommand>(serde_json::json!({
        "program": "",
        "args": []
    }))
    .is_err());
    assert!(serde_json::from_value::<CodexCommand>(serde_json::json!({
        "program": "cargo",
        "args": ["bad\u{0000}arg"]
    }))
    .is_err());

    let snapshot = RepoSnapshot::new(path);
    let plan = seiri_planner::plan_compatibility_safe_patches(&snapshot);
    let kernel = seiri_codex::build_review_kernel(&snapshot, &plan, None);
    let native = kernel.native_v2();
    assert!(native.actions[0]
        .command
        .args()
        .iter()
        .any(|argument| argument == path));
    let compatibility = kernel.compatibility_v1();
    assert!(compatibility.user_actions[0]
        .command
        .contains("'C:\\repo with ''quote''; $(Get-Item secret)'"));
}

#[test]
fn q19_query_and_linter_views_share_kernel_values() {
    let kernel = build_kernel("wording-lint-repo", ProfileKind::Common);
    let native = kernel.native_v2();
    let summary = kernel.query(CodexQueryKind::Summary);
    let routes = kernel.query(CodexQueryKind::Routes);
    let patches = kernel.query(CodexQueryKind::Patches);
    let linter_query = kernel.query(CodexQueryKind::Linter);
    let actions = kernel.query(CodexQueryKind::Actions);

    assert_eq!(summary.schema_version, CODEX_QUERY_SCHEMA_VERSION);
    let CodexQueryData::Summary(summary_data) = &summary.query else {
        panic!("summary query data");
    };
    assert_eq!(summary_data.canonical_claims, native.claims.len());
    assert_eq!(
        summary_data.canonical_route_assessments,
        native.route_assessments.len()
    );
    assert_eq!(summary_data.linter_findings, native.linter.findings.len());

    let CodexQueryData::Routes(route_data) = &routes.query else {
        panic!("routes query data");
    };
    assert_eq!(route_data.assessments, native.route_assessments);
    let CodexQueryData::Patches(patch_data) = &patches.query else {
        panic!("patch query data");
    };
    assert_eq!(patch_data, &native.plan);
    let CodexQueryData::Linter(linter_data) = &linter_query.query else {
        panic!("linter query data");
    };
    assert_eq!(linter_data, &kernel.linter_context());
    assert_eq!(
        linter_data.schema_version,
        CODEX_LINTER_CONTEXT_SCHEMA_VERSION
    );
    assert!(!linter_data.findings.is_empty());
    let CodexQueryData::Actions(action_data) = &actions.query else {
        panic!("actions query data");
    };
    assert_eq!(action_data, &native.actions);

    assert!(seiri_report::codex_native_to_markdown(&native)
        .contains("# RepoSeiri Codex Native Context"));
    assert!(seiri_report::codex_query_to_markdown(&routes).contains("# RepoSeiri Codex Query"));
    assert!(seiri_report::codex_linter_context_to_markdown(linter_data)
        .contains("# RepoSeiri Codex Linter Context"));
}
