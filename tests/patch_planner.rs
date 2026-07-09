use seiri_core::{GateKind, PatchOperationKind, ProfileKind};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn dry_run_plan_generates_safe_readme_docs_route_only_when_target_exists() {
    let snapshot =
        seiri_report::audit_repository_with_profile(fixture("safe-plan-repo"), ProfileKind::Common)
            .expect("audit fixture");
    let plan = seiri_planner::plan_safe_patches(&snapshot);

    assert_eq!(plan.mode, seiri_core::PatchPlanMode::DryRun);
    assert_eq!(plan.summary.safe_operations, 1);
    assert_eq!(plan.operations[0].gate, GateKind::Safe);
    assert_eq!(plan.operations[0].kind, PatchOperationKind::AddReadmeRoute);
    assert_eq!(plan.operations[0].path, "README.md");
    assert_eq!(plan.operations[0].pattern_id, "common.docs.route_present");
    assert!(plan.operations[0]
        .diff_preview
        .iter()
        .any(|line| line.contains("[Documentation](docs/)")));
}

#[test]
fn dry_run_plan_blocks_guarded_and_manual_items_without_generating_operations() {
    let snapshot = seiri_report::audit_repository_with_profile(
        fixture("missing-readme-repo"),
        ProfileKind::Library,
    )
    .expect("audit fixture");
    let plan = seiri_planner::plan_safe_patches(&snapshot);

    assert_eq!(plan.summary.safe_operations, 0);
    assert!(plan.summary.manual_items > 0);
    assert!(plan.summary.guarded_items > 0);
    assert!(plan
        .blocked
        .iter()
        .any(|item| item.gate == GateKind::Manual));
    assert!(plan
        .blocked
        .iter()
        .any(|item| item.gate == GateKind::Guarded));
}

#[test]
fn patch_plan_report_renders_json_and_markdown() {
    let plan =
        seiri_report::plan_repository_with_profile(fixture("safe-plan-repo"), ProfileKind::Common)
            .expect("plan fixture");
    let json = seiri_report::plan_to_json(&plan).expect("render plan JSON");
    let markdown = seiri_report::plan_to_markdown(&plan);

    assert!(json.contains("\"mode\": \"dry_run\""));
    assert!(json.contains("\"operations\""));
    assert!(markdown.contains("# RepoSeiri Patch Plan"));
    assert!(markdown.contains("## Safe Operations"));
    assert!(markdown.contains("## Blocked Items"));
}
