use seiri_core::{
    AggregateRepositoryEstimate, GateKind, PatchOperationKind, PatchPlanSource,
    PatchPreflightCheckKind, PatchPreflightStatus, PatchSafetyLevel, ProfileKind, ProfilePriority,
    RepoSnapshot, RouteKind, RouteState, Severity,
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
    assert_eq!(plan.planner_version, "safe_patch_planner.v4");
    assert!(plan.analysis_run.is_some());
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
    assert_eq!(
        plan.operations[0].proposal.schema_version,
        seiri_core::PATCH_PROPOSAL_SCHEMA_VERSION
    );
    assert_eq!(
        plan.operations[0].proposal.preflight_structure().decision,
        seiri_core::PatchProposalDecision::Ready
    );
    assert!(plan.operations[0].binding.is_some());
    assert!(plan.operations[0].preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::ExistingReadme
            && check.status == PatchPreflightStatus::Pass
    }));
    assert!(plan.operations[0].preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::ExistingTarget
            && check.status == PatchPreflightStatus::Pass
    }));
    assert!(plan.operations[0].preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::CurrentAnalysisInput
            && check.status == PatchPreflightStatus::Pass
    }));
    assert!(plan.operations[0].preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::AnchorContextBound
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
    assert!(json.contains("\"planner_version\": \"safe_patch_planner.v4\""));
    assert!(json.contains("\"schema_version\": \"seiri.patch_proposal.v1\""));
    assert!(json.contains("\"safety_policy\""));
    assert!(json.contains("\"preflight\""));
    assert!(json.contains("\"operations\""));
    assert!(markdown.contains("# RepoSeiri Patch Plan"));
    assert!(markdown.contains("Planner: `safe_patch_planner.v4`"));
    assert!(markdown.contains("Patch Proposal IR:"));
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
            calibration_estimate: Some(AggregateRepositoryEstimate::fixed(558_000, 1_000_000)),
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
    assert_eq!(
        blocked.suggested_kind,
        Some(PatchOperationKind::AddSecuritySkeletonDraft)
    );
    assert!(blocked.reason.contains("unsafe_to_invent"));
    assert!(blocked.preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::RouteSafeToInvent
            && check.status == PatchPreflightStatus::Blocked
    }));
}

#[test]
fn q8_expanded_operation_kinds_have_stable_json_names() {
    let kinds = [
        PatchOperationKind::AddReadmeRoute,
        PatchOperationKind::AddClaimBoundaryNote,
        PatchOperationKind::AddLifecycleRoute,
        PatchOperationKind::AddSupportSkeletonDraft,
        PatchOperationKind::AddSecuritySkeletonDraft,
        PatchOperationKind::MoveReadmeDetailToDocsDraft,
    ];

    let json = serde_json::to_string(&kinds).expect("operation kind JSON");

    assert_eq!(
        json,
        "[\"add_readme_route\",\"add_claim_boundary_note\",\"add_lifecycle_route\",\"add_support_skeleton_draft\",\"add_security_skeleton_draft\",\"move_readme_detail_to_docs_draft\"]"
    );
}

#[test]
fn q8_blocked_items_carry_review_required_operation_kinds() {
    let mut snapshot = RepoSnapshot::new("q8-boundary-fixture");
    push_priority(
        &mut snapshot,
        RouteKind::Support,
        GateKind::Guarded,
        "common.support.route_present",
    );
    push_priority(
        &mut snapshot,
        RouteKind::Security,
        GateKind::Manual,
        "common.security.route_present",
    );
    push_priority(
        &mut snapshot,
        RouteKind::Lifecycle,
        GateKind::Manual,
        "common.lifecycle.route_present",
    );
    push_priority(
        &mut snapshot,
        RouteKind::Release,
        GateKind::Guarded,
        "common.release.route_present",
    );
    push_priority(
        &mut snapshot,
        RouteKind::License,
        GateKind::Manual,
        "common.license.file_present",
    );

    let plan = seiri_planner::plan_safe_patches(&snapshot);

    assert!(plan.operations.is_empty());
    assert_blocked_kind(
        &plan,
        "common.support.route_present",
        PatchOperationKind::AddSupportSkeletonDraft,
        PatchSafetyLevel::ReviewRequired,
    );
    assert_blocked_kind(
        &plan,
        "common.security.route_present",
        PatchOperationKind::AddSecuritySkeletonDraft,
        PatchSafetyLevel::ManualOnly,
    );
    assert_blocked_kind(
        &plan,
        "common.lifecycle.route_present",
        PatchOperationKind::AddLifecycleRoute,
        PatchSafetyLevel::ManualOnly,
    );
    assert_blocked_kind(
        &plan,
        "common.release.route_present",
        PatchOperationKind::MoveReadmeDetailToDocsDraft,
        PatchSafetyLevel::ReviewRequired,
    );
    assert_blocked_kind(
        &plan,
        "common.license.file_present",
        PatchOperationKind::AddClaimBoundaryNote,
        PatchSafetyLevel::ManualOnly,
    );

    let markdown = seiri_report::plan_to_markdown(&plan);
    assert!(markdown.contains("Suggested kind: `AddSupportSkeletonDraft`"));
    assert!(markdown.contains("Suggested kind: `AddClaimBoundaryNote`"));
    assert!(markdown.contains("human policy"));
}

fn push_priority(snapshot: &mut RepoSnapshot, route: RouteKind, gate: GateKind, pattern_id: &str) {
    let index = snapshot.missing_route_priority.priorities.len() + 1;
    snapshot
        .missing_route_priority
        .priorities
        .push(seiri_core::MissingRoutePriority {
            rank: index,
            route,
            state: RouteState::Absent,
            gate,
            severity: Severity::Info,
            priority: ProfilePriority::Normal,
            priority_score_x100: 50,
            calibration_estimate: None,
            baseline_pattern_ids: vec![pattern_id.to_string()],
            candidate_pattern_ids: Vec::new(),
            co_occurrence_gap_ids: Vec::new(),
            evidence_ids: Vec::new(),
            reason: format!("{route:?} route needs review"),
        });
}

fn assert_blocked_kind(
    plan: &seiri_core::PatchPlan,
    pattern_id: &str,
    kind: PatchOperationKind,
    safety: PatchSafetyLevel,
) {
    let blocked = plan
        .blocked
        .iter()
        .find(|item| item.pattern_id == pattern_id)
        .expect("blocked item");

    assert_eq!(blocked.suggested_kind, Some(kind));
    assert_eq!(blocked.safety, safety);
    assert!(blocked.preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::SafeGate
            && check.status == PatchPreflightStatus::Blocked
            && check.detail.contains(&format!("{kind:?}"))
    }));
}
