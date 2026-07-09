use seiri_core::{
    GateKind, PatchOperationKind, PatchPlanSource, PatchPreflightCheckKind, PatchPreflightStatus,
    PatchSafetyLevel, ProfileKind, ProfilePriority, RepoSnapshot, RouteKind, RouteState, Severity,
};
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
    assert_eq!(plan.planner_version, "safe_patch_planner.v2");
    assert!(!plan.safety_policy.writes_files);
    assert!(!plan.safety_policy.applies_patches);
    assert!(plan.safety_policy.safe_gate_only);
    assert!(plan.safety_policy.requires_existing_targets);
    assert!(plan.safety_policy.blocks_unsafe_to_invent);
    assert!(plan.summary.total_candidates >= plan.summary.safe_operations + plan.blocked.len());
    assert_eq!(plan.summary.safe_operations, 1);
    assert_eq!(plan.summary.preview_only_operations, 1);
    assert!(plan.summary.preflight_passed > 0);
    assert_eq!(plan.operations[0].gate, GateKind::Safe);
    assert_eq!(plan.operations[0].kind, PatchOperationKind::AddReadmeRoute);
    assert_eq!(
        plan.operations[0].source,
        PatchPlanSource::ProfileRecommendation
    );
    assert_eq!(plan.operations[0].safety, PatchSafetyLevel::PreviewOnly);
    assert!(plan.operations[0].preview_only);
    assert!(plan.operations[0].requires_confirmation);
    assert_eq!(plan.operations[0].path, "README.md");
    assert_eq!(plan.operations[0].pattern_id, "common.docs.route_present");
    assert!(plan.operations[0].preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::ExistingReadme
            && check.status == PatchPreflightStatus::Pass
    }));
    assert!(plan.operations[0].preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::ExistingTarget
            && check.status == PatchPreflightStatus::Pass
    }));
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
    assert!(plan
        .blocked
        .iter()
        .any(|item| item.safety == PatchSafetyLevel::ManualOnly));
    assert!(plan.summary.preflight_failed > 0);
}

#[test]
fn patch_plan_report_renders_json_and_markdown() {
    let plan =
        seiri_report::plan_repository_with_profile(fixture("safe-plan-repo"), ProfileKind::Common)
            .expect("plan fixture");
    let json = seiri_report::plan_to_json(&plan).expect("render plan JSON");
    let markdown = seiri_report::plan_to_markdown(&plan);

    assert!(json.contains("\"mode\": \"dry_run\""));
    assert!(json.contains("\"planner_version\": \"safe_patch_planner.v2\""));
    assert!(json.contains("\"safety_policy\""));
    assert!(json.contains("\"preflight\""));
    assert!(json.contains("\"operations\""));
    assert!(markdown.contains("# RepoSeiri Patch Plan"));
    assert!(markdown.contains("Planner: `safe_patch_planner.v2`"));
    assert!(markdown.contains("Preflight:"));
    assert!(markdown.contains("## Safe Fixes"));
    assert!(markdown.contains("## Guarded Drafts"));
    assert!(markdown.contains("## Manual Decisions"));
}

#[test]
fn patch_plan_v2_adds_route_priority_review_items() {
    let snapshot = seiri_report::audit_repository_with_profile(
        fixture("readme-route-repo"),
        ProfileKind::Common,
    )
    .expect("audit fixture");
    let plan = seiri_planner::plan_safe_patches(&snapshot);

    let security_automation = plan
        .blocked
        .iter()
        .find(|item| {
            item.source == PatchPlanSource::MissingRoutePriority && item.pattern_id == "SEC-004"
        })
        .expect("route priority security automation candidate");
    assert_eq!(security_automation.gate, GateKind::Guarded);
    assert_eq!(security_automation.safety, PatchSafetyLevel::ReviewRequired);
    assert!(security_automation
        .reason
        .contains("Guarded recommendation requires maintainer confirmation"));
}

#[test]
fn patch_plan_v2_always_blocks_unsafe_to_invent_routes() {
    let mut snapshot = RepoSnapshot::new("unsafe-route-fixture");
    snapshot
        .missing_route_priority
        .priorities
        .push(seiri_core::MissingRoutePriority {
            rank: 1,
            route: RouteKind::Security,
            state: RouteState::UnsafeToInvent,
            gate: GateKind::Safe,
            severity: Severity::High,
            priority: ProfilePriority::Critical,
            priority_score_x100: 99,
            observed_missing_repositories: Some(558_000),
            observed_missing_x1000: Some(558),
            baseline_pattern_ids: vec!["common.security.route_present".to_string()],
            candidate_pattern_ids: Vec::new(),
            co_occurrence_gap_ids: Vec::new(),
            evidence_ids: Vec::new(),
            reason: "manual security route would invent disclosure policy".to_string(),
        });

    let plan = seiri_planner::plan_safe_patches(&snapshot);

    assert!(plan.operations.is_empty());
    let blocked = plan
        .blocked
        .iter()
        .find(|item| item.pattern_id == "common.security.route_present")
        .expect("unsafe route blocked item");
    assert_eq!(blocked.source, PatchPlanSource::MissingRoutePriority);
    assert!(blocked.reason.contains("unsafe_to_invent"));
    assert!(blocked.preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::RouteSafeToInvent
            && check.status == PatchPreflightStatus::Blocked
    }));
}
