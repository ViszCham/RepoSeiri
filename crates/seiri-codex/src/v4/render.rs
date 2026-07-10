use seiri_core::{CodexLinterContext, CodexNativeReviewContext, CodexQueryData, CodexQueryView};

#[must_use]
pub fn render_native_context_markdown(context: &CodexNativeReviewContext) -> String {
    let mut out = String::new();
    out.push_str("# RepoSeiri Codex Native Context\n\n");
    out.push_str(&format!("- Schema: `{}`\n", context.schema_version));
    out.push_str(&format!(
        "- Kernel: `{}`\n- Repository: `{}`\n",
        context.kernel_schema_version, context.repo_root
    ));
    match context.profile {
        Some(profile) => out.push_str(&format!("- Profile: `{profile}`\n")),
        None => out.push_str("- Profile: not selected\n"),
    }
    out.push_str(&format!(
        "- Canonical evidence facts: `{}`\n- Canonical route assessments: `{}`\n- Canonical claims: `{}`\n- Patch operations: `{}` / blocked `{}`\n- Linter findings: `{}`\n- Argv actions: `{}`\n- Boundary: {}\n\n",
        context.evidence_kernel.facts().len(),
        context.route_assessments.len(),
        context.claims.len(),
        context.plan.operations.len(),
        context.plan.blocked.len(),
        context.linter.findings.len(),
        context.actions.len(),
        context.claim_boundary
    ));

    out.push_str("## Native Routes\n\n");
    for assessment in &context.route_assessments {
        let readme = assessment.readme();
        out.push_str(&format!(
            "- `{:?}` root_structured `{}` inherited `{}` candidates `{}` local_present `{}` local_missing `{}` freshness `{:?}` policy `{:?}`\n",
            assessment.route(),
            assessment.presence().root_structured(),
            assessment.presence().inherited(),
            readme.routing().candidate_count(),
            readme.target_reachability().repository_local_present(),
            readme.target_reachability().repository_local_missing(),
            readme.freshness(),
            assessment.policy()
        ));
    }

    out.push_str("\n## Patch Proposals\n\n");
    if context.plan.operations.is_empty() {
        out.push_str("- No Ready Safe operations.\n");
    } else {
        for operation in &context.plan.operations {
            out.push_str(&format!(
                "- `{}` `{}` proposal `{}` schema `{}` edits `{}`\n",
                operation.id,
                operation.path,
                operation.proposal.id,
                operation.proposal.schema_version,
                operation.proposal.edits.len()
            ));
        }
    }

    out.push_str("\n## Argv Actions\n\n");
    for action in &context.actions {
        out.push_str(&format!(
            "- `{}` program `{}` argv `{:?}` mutates_files `{}`\n",
            action.id,
            action.command.program(),
            action.command.args(),
            action.mutates_files
        ));
    }
    out
}

#[must_use]
pub fn render_query_view_markdown(view: &CodexQueryView) -> String {
    let mut out = format!(
        "# RepoSeiri Codex Query\n\n- Schema: `{}`\n- Kernel: `{}`\n- Repository: `{}`\n- Query: `{:?}`\n- Boundary: {}\n\n",
        view.schema_version,
        view.kernel_schema_version,
        view.repo_root,
        view.query.kind(),
        view.claim_boundary
    );
    match &view.query {
        CodexQueryData::Summary(summary) => out.push_str(&format!(
            "- Entries: `{}`\n- Evidence facts: `{}`\n- Route assessments: `{}`\n- Claims: `{}`\n- Patch operations: `{}`\n- Blocked patch items: `{}`\n- Linter findings: `{}`\n",
            summary.audit.entries_scanned,
            summary.audit.evidence_facts,
            summary.canonical_route_assessments,
            summary.canonical_claims,
            summary.patch_operations,
            summary.blocked_patch_items,
            summary.linter_findings
        )),
        CodexQueryData::Routes(routes) => {
            for assessment in &routes.assessments {
                out.push_str(&format!(
                    "- `{:?}` presence `{:?}` readme `{:?}` policy `{:?}`\n",
                    assessment.route(),
                    assessment.presence(),
                    assessment.readme(),
                    assessment.policy()
                ));
            }
        }
        CodexQueryData::Patches(plan) => out.push_str(&format!(
            "- Safe operations: `{}`\n- Blocked items: `{}`\n- Planner: `{}`\n",
            plan.operations.len(),
            plan.blocked.len(),
            plan.planner_version
        )),
        CodexQueryData::Linter(linter) => out.push_str(&render_linter_context_markdown(linter)),
        CodexQueryData::Actions(actions) => {
            for action in actions {
                out.push_str(&format!(
                    "- `{}` program `{}` argv `{:?}`\n",
                    action.id,
                    action.command.program(),
                    action.command.args()
                ));
            }
        }
    }
    out
}

#[must_use]
pub fn render_linter_context_markdown(context: &CodexLinterContext) -> String {
    let mut out = format!(
        "# RepoSeiri Codex Linter Context\n\n- Schema: `{}`\n- Kernel: `{}`\n- Available: `{}`\n- Files scanned: `{}`\n- Generated surfaces: `{}`\n- Findings: `{}`\n- Suppressed boundary exceptions: `{}`\n- Boundary: {}\n\n",
        context.schema_version,
        context.kernel_schema_version,
        context.available,
        context.files_scanned,
        context.generated_surfaces,
        context.findings.len(),
        context.suppressed_boundary_exceptions,
        context.claim_boundary
    );
    for finding in &context.findings {
        out.push_str(&format!(
            "- `{}` `{}` line `{}` rule `{:?}` boundary `{:?}`\n",
            finding.id, finding.path, finding.line, finding.rule, finding.boundary
        ));
    }
    out
}
