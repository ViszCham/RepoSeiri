use seiri_core::{
    AnalysisScope, CoverageIncompleteReason, CoverageStatus, DeltaCompatibility, DeltaState,
    DeltaUnknownReason, PatchEditContent, PatchHoldReason, PatchProposalKind, ProfileKind,
    RouteKind,
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
    assert!(!json.contains("evidence_ids"));
    assert!(!json.contains("evrec-"));
    assert_eq!(first.schema_version, "seiri.portable-audit.v2");
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
fn route_delta_ignores_occurrence_only_line_shifts() {
    let before_root = fixture(
        "route-occurrence-before",
        "# Demo\n\n[Documentation](docs/)\n",
        true,
    );
    let after_root = fixture(
        "route-occurrence-after",
        "# Demo\n\nUnrelated introduction.\n\n[Documentation](docs/)\n",
        true,
    );
    let before =
        seiri_report::audit_repository_with_profile(&before_root, ProfileKind::Common).unwrap();
    let after =
        seiri_report::audit_repository_with_profile(&after_root, ProfileKind::Common).unwrap();
    let before = seiri_delta::portable_snapshot(&before).unwrap();
    let after = seiri_delta::portable_snapshot(&after).unwrap();

    let before_docs = before
        .routes
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    let after_docs = after
        .routes
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    assert_eq!(
        before_docs
            .evidence
            .iter()
            .map(|item| (item.identity, item.state))
            .collect::<Vec<_>>(),
        after_docs
            .evidence
            .iter()
            .map(|item| (item.identity, item.state))
            .collect::<Vec<_>>()
    );
    assert_ne!(
        before_docs
            .evidence
            .iter()
            .map(|item| item.occurrence)
            .collect::<Vec<_>>(),
        after_docs
            .evidence
            .iter()
            .map(|item| item.occurrence)
            .collect::<Vec<_>>()
    );

    let delta = seiri_delta::compare(&before, &after);
    let docs = delta
        .routes
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    assert_eq!(docs.state, DeltaState::Unchanged);
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
    after.configuration.calibration_binding = Some("overlay-b".to_string());
    after.digest.configuration = seiri_core::Digest32::new([9; 32]);
    let serialized = serde_json::to_string(&after).unwrap();
    assert!(!serialized.contains("private raw calibration value"));
    assert_eq!(
        seiri_delta::compare(&before, &after).compatibility,
        DeltaCompatibility::Unknown(DeltaUnknownReason::UnknownPrivateBinding)
    );
    cleanup(root);
}

#[test]
fn patch_plan_only_links_existing_targets_and_binding_rejects_stale_bytes() {
    let root = fixture("planner", "# Demo\n", true);
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let plan = seiri_planner::plan_patches(&snapshot);
    let operation = plan
        .operations
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .unwrap();
    assert_eq!(operation.target_path, "docs/");
    assert_eq!(operation.decision_basis.gate, seiri_core::GateKind::Safe);
    assert!(!operation.decision_basis.claim_ids.is_empty());
    assert!(!operation.decision_basis.evidence_fingerprints.is_empty());
    assert_eq!(
        operation.decision_basis.claim_semantic_revision,
        seiri_core::CLAIM_SEMANTIC_REVISION
    );
    assert_eq!(
        operation.decision_basis.planner_semantic_revision,
        "seiri.patch-planner.v5"
    );
    assert!(!plan.writes_files);
    assert_eq!(
        plan.proposal_count(PatchProposalKind::EditExisting),
        plan.operations.len()
    );
    assert!(plan.held.iter().any(|hold| {
        hold.route == RouteKind::Support
            && hold.proposal_kind() == PatchProposalKind::CreateSkeleton
    }));
    assert!(plan.held.iter().any(|hold| {
        hold.route == RouteKind::Security
            && hold.proposal_kind() == PatchProposalKind::ManualDecision
    }));
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
fn patch_plan_holds_ambiguous_mixed_language_readme() {
    let root = fixture("paired-ambiguous", "# 日本語 and English\n\nMixed.\n", true);
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let plan = seiri_planner::plan_patches(&snapshot);
    assert!(!plan
        .operations
        .iter()
        .any(|item| item.route == RouteKind::Docs));
    assert!(plan.held.iter().any(|item| {
        item.route == RouteKind::Docs && item.reason == PatchHoldReason::PairedLanguageIncomplete
    }));
    cleanup(root);
}

#[test]
fn patch_plan_emits_localized_edits_for_japanese_first_english_second_readme() {
    let root = fixture(
        "paired-sections",
        "# Demo\n\n## 日本語\n\n説明。\n\n### 詳細\n\n本文。\n\n## English\n\nDescription.\n\n### Details\n\nBody.\n",
        true,
    );
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();
    let plan = seiri_planner::plan_patches(&snapshot);
    let docs = plan
        .operations
        .iter()
        .find(|item| item.route == RouteKind::Docs)
        .expect("paired docs operation");
    assert!(docs.paired_language);
    assert_eq!(docs.proposal.edits.len(), 2);
    let replacements = docs
        .proposal
        .edits
        .iter()
        .filter_map(|edit| match &edit.content {
            PatchEditContent::Literal(value) => Some(value.as_str()),
            PatchEditContent::UnresolvedSlot(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(replacements
        .iter()
        .any(|value| value.contains("ドキュメント")));
    assert!(replacements
        .iter()
        .any(|value| value.contains("Documentation")));
    cleanup(root);
}

#[test]
fn patch_plan_holds_conflicting_and_unknown_target_relations() {
    let root = fixture("relation-hold", "# Demo\n", false);
    fs::write(root.join("SECURITY.md"), "# Security\n").unwrap();
    let mut snapshot =
        seiri_report::audit_repository_with_profile(&root, ProfileKind::Common).unwrap();

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let conflict_source =
        seiri_report::audit_repository_subtree(fixture_root.join("conflicting-canonical-targets"))
            .unwrap();
    snapshot.document_consistency.conflicts.push(
        conflict_source
            .document_consistency
            .conflicts
            .iter()
            .find(|item| item.route == RouteKind::Security)
            .unwrap()
            .clone(),
    );
    let conflict_plan = seiri_planner::plan_patches(&snapshot);
    assert!(conflict_plan.held.iter().any(|item| {
        item.route == RouteKind::Security && item.reason == PatchHoldReason::CanonicalConflict
    }));

    snapshot.document_consistency.conflicts.clear();
    let relation_source =
        seiri_report::audit_repository_subtree(fixture_root.join("related-route-targets")).unwrap();
    let mut relation = relation_source
        .document_consistency
        .relations
        .iter()
        .find(|item| item.route == RouteKind::Security)
        .unwrap()
        .clone();
    relation.relation = seiri_core::TargetRelation::Unknown;
    snapshot.document_consistency.relations.push(relation);
    let unknown_plan = seiri_planner::plan_patches(&snapshot);
    assert!(unknown_plan.held.iter().any(|item| {
        item.route == RouteKind::Security && item.reason == PatchHoldReason::UnknownTargetRelation
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
