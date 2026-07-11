use seiri_core::{PatchProposalDecision, PatchProposalIssueKind, ProfileKind};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn patch_plan_links_only_existing_targets_and_is_stale_bound() {
    let root = temp_root("patch-plan");
    fs::create_dir(root.join("docs")).expect("docs");
    fs::write(root.join("README.md"), "# Tool\n").expect("README");
    fs::write(root.join("docs/README.md"), "# Docs\n").expect("docs");

    let analysis =
        seiri_report::audit_repository_with_profile(&root, ProfileKind::Library).expect("audit");
    let plan = seiri_planner::plan_patches(&analysis);
    assert_eq!(plan.schema_version, "seiri.patch-plan.v1");
    assert!(!plan.writes_files);
    let docs = plan
        .operations
        .iter()
        .find(|operation| operation.target_path.starts_with("docs"))
        .expect("docs operation");
    let current = fs::read(root.join("README.md")).expect("README bytes");
    assert_eq!(
        docs.binding
            .preflight_against(&docs.proposal, &current)
            .decision,
        PatchProposalDecision::Ready
    );

    let mut stale = current;
    stale.extend_from_slice(b"\nchanged\n");
    let preflight = docs.binding.preflight_against(&docs.proposal, &stale);
    assert_ne!(preflight.decision, PatchProposalDecision::Ready);
    assert!(preflight
        .issues
        .iter()
        .any(|issue| issue.kind == PatchProposalIssueKind::StaleBase));
    fs::remove_dir_all(root).expect("remove temp repository");
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-{label}-{nonce}"));
    fs::create_dir_all(&root).expect("temp root");
    root
}
