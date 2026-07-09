use seiri_core::{
    stable_id, CodexAuditSummary, CodexBlockedDigest, CodexFindingDigest, CodexPatchDigest,
    CodexPrDraft, CodexReviewContext, CodexUserAction, PatchPlan, ProfileKind, RepoSnapshot,
    SCHEMA_VERSION, TOOL_NAME,
};

#[must_use]
pub fn build_review_context(snapshot: &RepoSnapshot, plan: &PatchPlan) -> CodexReviewContext {
    let audit = audit_summary(snapshot);
    let findings = snapshot
        .findings
        .iter()
        .map(|finding| CodexFindingDigest {
            id: finding.id.clone(),
            severity: finding.severity,
            title: finding.title.clone(),
            gate: finding
                .recommendation
                .as_ref()
                .map(|recommendation| recommendation.gate),
            recommendation: finding
                .recommendation
                .as_ref()
                .map(|recommendation| recommendation.message.clone()),
        })
        .collect::<Vec<_>>();
    let safe_operations = plan
        .operations
        .iter()
        .map(|operation| CodexPatchDigest {
            id: operation.id.clone(),
            gate: operation.gate,
            kind: operation.kind,
            path: operation.path.clone(),
            title: operation.title.clone(),
            planned_change: operation.planned_change.clone(),
        })
        .collect::<Vec<_>>();
    let blocked_items = plan
        .blocked
        .iter()
        .map(|item| CodexBlockedDigest {
            id: item.id.clone(),
            gate: item.gate,
            pattern_id: item.pattern_id.clone(),
            title: item.title.clone(),
            reason: item.reason.clone(),
        })
        .collect::<Vec<_>>();
    let profile = snapshot.profile.as_ref().map(|profile| profile.profile);
    let pr_draft = build_pr_draft(snapshot, plan, &audit);
    let user_actions = build_user_actions(snapshot, profile);

    CodexReviewContext {
        schema_version: SCHEMA_VERSION.to_string(),
        tool: TOOL_NAME.to_string(),
        repo_root: snapshot.repo_root.clone(),
        profile,
        audit,
        plan: plan.summary,
        findings,
        safe_operations,
        blocked_items,
        user_actions,
        pr_draft,
        claim_boundary: "Codex context is generated from RepoSeiri Rust core outputs. It is a draft review artifact only; it does not create branches, write files, call GitHub, open PRs, adopt policies, or guarantee popularity, trust, security, or quality.".to_string(),
    }
}

#[must_use]
pub fn render_review_context_markdown(context: &CodexReviewContext) -> String {
    let mut out = String::new();
    out.push_str("# RepoSeiri Codex Review Context\n\n");
    out.push_str(&format!("- Schema: `{}`\n", context.schema_version));
    out.push_str(&format!("- Repository: `{}`\n", context.repo_root));
    match context.profile {
        Some(profile) => out.push_str(&format!("- Profile: `{profile}`\n")),
        None => out.push_str("- Profile: not selected\n"),
    }
    if let Some(score) = context.audit.profile_score_x100 {
        out.push_str(&format!("- Profile score view: `{score}` / `100`\n"));
    }
    out.push_str(&format!("- Findings: `{}`\n", context.audit.findings));
    out.push_str(&format!(
        "- Safe operations: `{}`\n",
        context.plan.safe_operations
    ));
    out.push_str(&format!(
        "- Blocked items: `{}`\n",
        context.blocked_items.len()
    ));
    out.push_str(&format!("- Boundary: {}\n\n", context.claim_boundary));

    out.push_str("## User Actions\n\n");
    for action in &context.user_actions {
        out.push_str(&format!("### {}\n\n", action.label));
        out.push_str(&format!("- Mutates files: `{}`\n", action.mutates_files));
        out.push_str(&format!(
            "- Requires confirmation: `{}`\n",
            action.requires_confirmation
        ));
        out.push_str(&format!("- Detail: {}\n", action.detail));
        out.push_str("```powershell\n");
        out.push_str(&action.command);
        out.push('\n');
        out.push_str("```\n\n");
    }

    out.push_str("## PR Draft\n\n");
    out.push_str(&format!("- Title: {}\n", context.pr_draft.title));
    out.push_str(&format!("- Draft: `{}`\n", context.pr_draft.draft));
    out.push_str(&format!(
        "- Labels: `{}`\n\n",
        context.pr_draft.labels.join("`, `")
    ));
    out.push_str(&context.pr_draft.body);
    out.push('\n');

    out
}

#[must_use]
pub fn render_pr_body(context: &CodexReviewContext) -> String {
    context.pr_draft.body.clone()
}

fn audit_summary(snapshot: &RepoSnapshot) -> CodexAuditSummary {
    let baseline = snapshot.baseline.as_ref();
    CodexAuditSummary {
        entries_scanned: snapshot.entry_count,
        evidence_items: snapshot.evidence.len(),
        findings: snapshot.findings.len(),
        pattern_matches: snapshot.pattern_matches.len(),
        profile_score_x100: snapshot
            .profile
            .as_ref()
            .map(|profile| profile.score.score_x100),
        required_present: baseline.map(|baseline| baseline.summary.required_present),
        required_missing: baseline.map(|baseline| baseline.summary.required_missing),
        optional_present: baseline.map(|baseline| baseline.summary.optional_present),
        optional_missing: baseline.map(|baseline| baseline.summary.optional_missing),
    }
}

fn build_pr_draft(
    snapshot: &RepoSnapshot,
    plan: &PatchPlan,
    audit: &CodexAuditSummary,
) -> CodexPrDraft {
    let mut body = String::new();
    body.push_str("## Summary\n\n");
    body.push_str("- Generated from RepoSeiri Rust core audit and dry-run plan output.\n");
    body.push_str(&format!(
        "- Scanned `{}` entries and collected `{}` evidence items.\n",
        audit.entries_scanned, audit.evidence_items
    ));
    body.push_str(&format!("- Findings: `{}`.\n", audit.findings));
    body.push_str(&format!(
        "- Safe dry-run operations: `{}`.\n\n",
        plan.summary.safe_operations
    ));

    body.push_str("## Safe Patch Plan\n\n");
    if plan.operations.is_empty() {
        body.push_str("- No safe operations were generated.\n\n");
    } else {
        for operation in &plan.operations {
            body.push_str(&format!(
                "- `{}` `{}`: {}\n",
                operation.id, operation.path, operation.planned_change
            ));
        }
        body.push('\n');
    }

    body.push_str("## Review Required\n\n");
    if plan.blocked.is_empty() {
        body.push_str("- No guarded or manual items are blocked in this plan.\n\n");
    } else {
        for item in &plan.blocked {
            body.push_str(&format!(
                "- `{}` `{:?}` `{}`: {}\n",
                item.id, item.gate, item.pattern_id, item.reason
            ));
        }
        body.push('\n');
    }

    body.push_str("## Verification\n\n");
    body.push_str("- [ ] Review generated safe operations before applying them.\n");
    body.push_str("- [ ] Confirm guarded and manual decisions with maintainers.\n");
    body.push_str("- [ ] Run `cargo test --workspace` after any applied changes.\n\n");

    body.push_str("## Boundary\n\n");
    body.push_str("- This PR body is a draft generated by RepoSeiri. It does not claim popularity, trust, security, quality, or external validation outcomes.\n");
    body.push_str("- RepoSeiri did not create this PR, push a branch, call GitHub, or mutate repository files while generating this context.\n");

    let profile = snapshot
        .profile
        .as_ref()
        .map_or(ProfileKind::Common, |profile| profile.profile);
    CodexPrDraft {
        title: format!("RepoSeiri: review repository trust routes for {profile}"),
        body,
        labels: vec![
            "reposeiri".to_string(),
            "repository-quality".to_string(),
            "draft".to_string(),
        ],
        draft: true,
    }
}

fn build_user_actions(
    snapshot: &RepoSnapshot,
    profile: Option<ProfileKind>,
) -> Vec<CodexUserAction> {
    let selected_profile = profile.unwrap_or(ProfileKind::Common);
    let path = shell_quote(&snapshot.repo_root);
    vec![
        CodexUserAction {
            id: stable_id("codex-action", 1),
            label: "Render audit report".to_string(),
            command: format!(
                "cargo run --quiet -p seiri-cli -- audit --path {path} --profile {selected_profile} --format markdown"
            ),
            mutates_files: false,
            requires_confirmation: false,
            detail: "Re-run the Rust core audit and inspect evidence, baseline, profile, and findings.".to_string(),
        },
        CodexUserAction {
            id: stable_id("codex-action", 2),
            label: "Render dry-run patch plan".to_string(),
            command: format!(
                "cargo run --quiet -p seiri-cli -- plan --path {path} --profile {selected_profile} --format markdown"
            ),
            mutates_files: false,
            requires_confirmation: false,
            detail: "Show safe operations and guarded/manual blocked items without writing files.".to_string(),
        },
        CodexUserAction {
            id: stable_id("codex-action", 3),
            label: "Render Codex PR draft context".to_string(),
            command: format!(
                "cargo run --quiet -p seiri-cli -- codex --path {path} --profile {selected_profile} --format markdown"
            ),
            mutates_files: false,
            requires_confirmation: false,
            detail: "Generate the Codex-facing review context and draft PR body from Rust core outputs.".to_string(),
        },
    ]
}

fn shell_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}
