use seiri_core::{ClaimBoundaryKind, GateKind, MeaningAtom, ProfileKind, RouteKind};
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

    assert_eq!(context.schema_version, "seiri.block_p.v1");
    assert_eq!(context.profile, Some(ProfileKind::Common));
    assert_eq!(context.plan.safe_operations, 1);
    assert_eq!(context.audit.profile_branches, 9);
    assert!(context.audit.top_profile.is_some());
    assert!(context.audit.top_profile_confidence_x100.is_some());
    assert!(context.audit.missing_route_priorities > 0);
    assert!(context.audit.co_occurrence_gaps > 0);
    assert!(context.audit.top_missing_route.is_some());
    assert!(context.audit.top_missing_route_priority_x100.is_some());
    assert_eq!(context.safe_operations.len(), 1);
    assert!(!context.blocked_items.is_empty());
    assert!(context.route_review.strong_routes > 0);
    assert!(context.route_review.missing_routes > 0);
    assert_eq!(context.claims.total, context.audit.content_claims);
    assert!(context.claims.routes_with_claims > 0);
    assert!(context.claims.evidence_linked_claims > 0);
    assert!(context
        .claims
        .boundary_kinds
        .contains(&ClaimBoundaryKind::NotQualityGuarantee));
    assert!(context.wording_lint.available);
    assert_eq!(context.wording_lint.generated_surfaces, 3);
    assert_eq!(context.route_meanings.len(), context.routes.len());
    assert_eq!(
        context.route_review.safe_fixes,
        context.safe_operations.len()
    );
    assert_eq!(
        context.route_review.guarded_drafts,
        context.blocked_items.len()
    );
    assert!(context.route_review.manual_decisions > 0);
    assert!(!context.routes.is_empty());
    assert!(!context.co_occurrence_gaps.is_empty());
    assert!(context
        .blocked_items
        .iter()
        .all(|item| item.gate == GateKind::Guarded));
    assert!(context
        .pr_draft
        .body
        .contains("Generated from RepoSeiri Rust core audit"));
    assert!(context.pr_draft.body.contains("## Route Review"));
    assert!(context.pr_draft.body.contains("## Safe Fixes"));
    assert!(context.pr_draft.body.contains("## Guarded Drafts"));
    assert!(context.pr_draft.body.contains("## Manual Decisions"));
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
    assert!(context.user_actions.iter().all(|action| {
        !action.command.contains("git ")
            && !action.command.contains("gh ")
            && !action.command.contains("api.github")
    }));
    assert!(context
        .user_actions
        .iter()
        .any(|action| action.command.contains("seiri-cli -- codex")));
    assert!(markdown.contains("# RepoSeiri Codex Review Context"));
    assert!(markdown.contains("## Claim Summary"));
    assert!(markdown.contains("## Wording Lint"));
    assert!(markdown.contains("## Route Meaning Digest"));
    assert!(markdown.contains("## Route Review"));
    assert!(markdown.contains("## Co-occurrence Gaps"));
    assert!(markdown.contains("## PR Draft"));
    assert!(body.contains("## Claim / Wording / Meaning Digest"));
    assert!(body.contains("## Guarded Drafts"));
    assert!(body.contains("withheld from Codex actionable context"));
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
    assert!(json.contains("\"claims\""));
    assert!(json.contains("\"wording_lint\""));
    assert!(json.contains("\"route_meanings\""));
    assert!(json.contains("\"route_review\""));
    assert!(json.contains("\"routes\""));
    assert!(json.contains("\"co_occurrence_gaps\""));
    assert!(!json.contains("\"gate\": \"manual\""));
    assert!(pr_json.contains("\"draft\": true"));
    assert!(pr_json.contains("RepoSeiri"));
}

#[test]
fn q5_codex_context_binds_route_digests_to_claim_ids() {
    let context = seiri_report::codex_repository_with_profile(
        fixture("readme-route-repo"),
        ProfileKind::Common,
    )
    .expect("codex context");

    let docs_route = context
        .routes
        .iter()
        .find(|route| route.route == RouteKind::Docs)
        .expect("docs route digest");
    assert!(!docs_route.claim_ids.is_empty());
    assert!(docs_route
        .claim_ids
        .iter()
        .all(|id| id.starts_with("claim-")));
    assert!(docs_route
        .boundary_kinds
        .contains(&ClaimBoundaryKind::NotQualityGuarantee));

    let markdown = seiri_report::codex_to_markdown(&context);
    let body = seiri_report::codex_pr_body_to_markdown(&context);
    let json = seiri_report::codex_to_json(&context).expect("codex JSON");

    assert!(context.claim_boundary.contains("claim id"));
    assert!(markdown.contains("claims `claim-"));
    assert!(markdown.contains("boundary_kinds `"));
    assert!(body.contains("claims `claim-"));
    assert!(json.contains("\"claim_ids\""));
    assert!(json.contains("\"boundary_kinds\""));
}

#[test]
fn q10_codex_context_carries_claim_wording_and_route_meaning_digests() {
    let context = seiri_report::codex_repository_with_profile(
        fixture("wording-lint-repo"),
        ProfileKind::Common,
    )
    .expect("codex context");

    assert!(context.claims.total > 0);
    assert_eq!(
        context.claims.observed
            + context.claims.inferred
            + context.claims.suggested
            + context.claims.blocked,
        context.claims.total
    );
    assert!(context.wording_lint.available);
    assert!(context.wording_lint.findings >= 4);
    assert!(context
        .wording_lint
        .boundary_kinds
        .contains(&ClaimBoundaryKind::NotSecurityGuarantee));

    let docs_meaning = context
        .route_meanings
        .iter()
        .find(|meaning| meaning.route == RouteKind::Docs)
        .expect("docs route meaning digest");
    assert!(docs_meaning
        .does_not_indicate
        .contains(&ClaimBoundaryKind::NotQualityGuarantee));
    assert!(context.route_meanings.iter().any(|meaning| meaning
        .indicates
        .contains(&MeaningAtom::HumanReviewRequired)));

    let markdown = seiri_report::codex_to_markdown(&context);
    let body = seiri_report::codex_pr_body_to_markdown(&context);
    let json = seiri_report::codex_to_json(&context).expect("codex JSON");

    assert!(markdown.contains("Wording lint findings"));
    assert!(markdown.contains("does_not_indicate"));
    assert!(body.contains("Route meaning digests emitted"));
    assert!(body.contains("RepoSeiri did not create this PR, push a branch, call GitHub"));
    assert!(json.contains("\"available\": true"));
    assert!(json.contains("\"does_not_indicate\""));
}
