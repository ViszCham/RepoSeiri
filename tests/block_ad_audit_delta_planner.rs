use seiri_core::{
    AnalysisScope, BilingualStructuralPair, CoverageIncompleteReason, CoverageStatus,
    DeltaCompatibility, DeltaState, DeltaUnknownReason, PlannerV5HoldReason, ProfileKind,
    RouteKind, SourceSpan,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn portable_snapshot_is_deterministic_and_excludes_source_text() {
    let root = fixture(
        "portable",
        "# PRIVATE-SOURCE-SENTINEL\n\nLocal prose.\n",
        true,
    );
    let first = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let second = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let first = seiri_delta::portable_snapshot(&first).unwrap();
    let second = seiri_delta::portable_snapshot(&second).unwrap();
    assert_eq!(first, second);
    let json = serde_json::to_string(&first).unwrap();
    assert!(!json.contains("PRIVATE-SOURCE-SENTINEL"));
    assert!(json.contains("sha256:"));
    cleanup(root);
}

#[test]
fn incompatible_scope_or_configuration_yields_unknown_without_deltas() {
    let root = fixture("compatibility", "# Demo\n", false);
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let before = seiri_delta::portable_snapshot(&snapshot).unwrap();

    let mut scope_changed = before.clone();
    scope_changed.configuration.scope = AnalysisScope::Subtree;
    let scope_delta = seiri_delta::compare(&before, &scope_changed);
    assert_eq!(
        scope_delta.compatibility,
        DeltaCompatibility::Unknown(DeltaUnknownReason::ScopeMismatch)
    );
    assert!(scope_delta.routes.is_empty());
    assert!(scope_delta.improvements.is_empty());

    let mut config_changed = before.clone();
    config_changed.digest.configuration = seiri_core::Digest32::new([7; 32]);
    let config_delta = seiri_delta::compare(&before, &config_changed);
    assert_eq!(
        config_delta.compatibility,
        DeltaCompatibility::Unknown(DeltaUnknownReason::ConfigurationMismatch)
    );
    assert!(config_delta.regressions.is_empty());
    cleanup(root);
}

#[test]
fn complete_route_removal_is_a_regression_but_partial_to_absent_is_unknown() {
    let before_root = fixture("route-before", "# Demo\n\n[Documentation](docs/)\n", true);
    let after_root = fixture("route-after", "# Demo\n", true);
    let before =
        seiri_report::audit_repository_with_profile(&before_root, ProfileKind::Common).unwrap();
    let after =
        seiri_report::audit_repository_with_profile(&after_root, ProfileKind::Common).unwrap();
    let before = seiri_delta::portable_snapshot(&before).unwrap();
    let after = seiri_delta::portable_snapshot(&after).unwrap();
    let delta = seiri_delta::compare(&before, &after);
    let docs = delta
        .routes
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    assert_eq!(docs.state, DeltaState::Removed);
    assert!(delta
        .regressions
        .iter()
        .any(|item| item.domain == "route" && item.key == "Docs"));
    let improvement = seiri_delta::compare(&after, &before);
    assert!(improvement
        .improvements
        .iter()
        .any(|item| item.domain == "route" && item.key == "Docs"));

    let mut partial = before.clone();
    let partial_docs = partial
        .routes
        .iter_mut()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    partial_docs.coverage = CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded);
    let partial_delta = seiri_delta::compare(&partial, &after);
    let docs = partial_delta
        .routes
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    assert_eq!(docs.state, DeltaState::Unknown);
    assert!(!partial_delta
        .regressions
        .iter()
        .any(|item| item.key == "Docs"));
    cleanup(before_root);
    cleanup(after_root);
}

#[test]
fn redacted_private_overlay_identity_changes_configuration_only() {
    let root = fixture("private-overlay", "# Demo\n", false);
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let before = seiri_delta::portable_snapshot(&snapshot).unwrap();
    let mut after = before.clone();
    after.configuration.visibility = seiri_core::AnalysisVisibility::LocalPrivateCalibration;
    after.configuration.redacted_calibration_fingerprint = Some("redacted:overlay-b".to_string());
    after.digest.configuration = seiri_core::Digest32::new([9; 32]);
    let serialized = serde_json::to_string(&after).unwrap();
    assert!(!serialized.contains("private raw calibration value"));
    assert_eq!(
        seiri_delta::compare(&before, &after).compatibility,
        DeltaCompatibility::Unknown(DeltaUnknownReason::ConfigurationMismatch)
    );
    cleanup(root);
}

#[test]
fn planner_v5_only_links_existing_targets_and_binding_rejects_stale_bytes() {
    let root = fixture("planner", "# Demo\n", true);
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let plan = seiri_planner::plan_existing_route_links(&snapshot);
    let operation = plan
        .operations
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    assert_eq!(operation.target_path, "docs/");
    assert!(!plan.writes_files);
    let stale = b"# Demo changed\n";
    assert_ne!(
        operation
            .binding
            .preflight_against(&operation.proposal, stale)
            .decision,
        seiri_core::PatchProposalDecision::Ready
    );
    assert_eq!(fs::read(root.join("README.md")).unwrap(), b"# Demo\n");
    cleanup(root);
}

#[test]
fn planner_v5_holds_both_language_insertions_when_one_anchor_is_invalid() {
    let root = fixture("paired", "# Japanese\n\n# English\n", true);
    let mut snapshot =
        seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    snapshot
        .route_content_v2
        .structural_pairs
        .push(BilingualStructuralPair {
            document_path: "README.md".to_string(),
            left_heading: SourceSpan::new(1, 1, 0, 10),
            right_heading: SourceSpan::new(3, 1, 12, 10_000),
            normalized_targets: Vec::new(),
            evidence_ids: Vec::new(),
            candidate_only: true,
        });
    let plan = seiri_planner::plan_existing_route_links(&snapshot);
    assert!(!plan
        .operations
        .iter()
        .any(|item| item.route == RouteKind::Docs));
    assert!(plan.held.iter().any(|item| {
        item.route == RouteKind::Docs
            && item.reason == PlannerV5HoldReason::PairedLanguageIncomplete
    }));
    cleanup(root);
}

#[test]
fn planner_v5_holds_conflicting_and_unknown_target_relations() {
    let root = fixture("relation-hold", "# Demo\n", false);
    fs::write(root.join("SECURITY.md"), "# Security\n").unwrap();
    let mut snapshot =
        seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let conflict_source =
        seiri_report::audit_repository_subtree(fixture_root.join("block-x-true-conflict")).unwrap();
    snapshot.document_consistency.conflicts.push(
        conflict_source
            .document_consistency
            .conflicts
            .iter()
            .find(|item| item.route == RouteKind::Security)
            .unwrap()
            .clone(),
    );
    let conflict_plan = seiri_planner::plan_existing_route_links(&snapshot);
    assert!(conflict_plan.held.iter().any(|item| {
        item.route == RouteKind::Security && item.reason == PlannerV5HoldReason::CanonicalConflict
    }));

    snapshot.document_consistency.conflicts.clear();
    let relation_source =
        seiri_report::audit_repository_subtree(fixture_root.join("block-x-related-targets"))
            .unwrap();
    let mut relation = relation_source
        .document_consistency
        .relations
        .iter()
        .find(|item| item.route == RouteKind::Security)
        .unwrap()
        .clone();
    relation.relation = seiri_core::TargetRelation::Unknown;
    snapshot.document_consistency.relations.push(relation);
    let unknown_plan = seiri_planner::plan_existing_route_links(&snapshot);
    assert!(unknown_plan.held.iter().any(|item| {
        item.route == RouteKind::Security
            && item.reason == PlannerV5HoldReason::UnknownTargetRelation
    }));
    cleanup(root);
}

fn fixture(name: &str, readme: &str, docs: bool) -> PathBuf {
    let root = temp_root(name);
    fs::write(root.join("README.md"), readme).unwrap();
    if docs {
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/index.md"), "# Docs\n").unwrap();
    }
    root
}

fn temp_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-ad-{name}-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    root
}

fn cleanup(path: impl AsRef<Path>) {
    fs::remove_dir_all(path).unwrap();
}
