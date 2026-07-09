use seiri_core::ProfileKind;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn codex_context_uses_audit_and_plan_outputs_for_pr_draft() {
    let context =
        seiri_report::codex_repository_with_profile(fixture("safe-plan-repo"), ProfileKind::Common)
            .expect("codex context");

    assert_eq!(context.schema_version, "seiri.block_f.v1");
    assert_eq!(context.profile, Some(ProfileKind::Common));
    assert_eq!(context.plan.safe_operations, 1);
    assert_eq!(context.safe_operations.len(), 1);
    assert!(!context.blocked_items.is_empty());
    assert!(context
        .pr_draft
        .body
        .contains("Generated from RepoSeiri Rust core audit"));
    assert!(context.pr_draft.body.contains("## Safe Patch Plan"));
    assert!(context.claim_boundary.contains("does not create branches"));
}

#[test]
fn codex_context_renders_user_actions_without_mutation() {
    let context =
        seiri_report::codex_repository_with_profile(fixture("safe-plan-repo"), ProfileKind::Common)
            .expect("codex context");
    let markdown = seiri_report::codex_to_markdown(&context);
    let body = seiri_report::codex_pr_body_to_markdown(&context);

    assert!(context.user_actions.len() >= 3);
    assert!(context
        .user_actions
        .iter()
        .all(|action| !action.mutates_files));
    assert!(context
        .user_actions
        .iter()
        .any(|action| action.command.contains("seiri-cli -- codex")));
    assert!(markdown.contains("# RepoSeiri Codex Review Context"));
    assert!(markdown.contains("## PR Draft"));
    assert!(body.contains("## Review Required"));
}

#[test]
fn codex_context_json_contains_pr_draft_surface() {
    let context = seiri_report::codex_repository_with_profile(
        fixture("missing-readme-repo"),
        ProfileKind::Library,
    )
    .expect("codex context");
    let json = seiri_report::codex_to_json(&context).expect("codex JSON");
    let pr_json = seiri_report::codex_pr_draft_to_json(&context).expect("PR draft JSON");

    assert!(json.contains("\"pr_draft\""));
    assert!(json.contains("\"user_actions\""));
    assert!(pr_json.contains("\"draft\": true"));
    assert!(pr_json.contains("RepoSeiri"));
}
