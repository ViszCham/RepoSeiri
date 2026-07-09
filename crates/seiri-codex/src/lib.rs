use seiri_core::{
    route_meaning_rule, stable_id, CalibrationRun, CalibrationSourceVisibilitySummary,
    ClaimBoundaryKind, ClaimId, ClaimRefIndex, ClaimStrength, CodexAuditSummary,
    CodexBlockedDigest, CodexClaimSummary, CodexCoOccurrenceDigest, CodexFindingDigest,
    CodexPatchDigest, CodexPrDraft, CodexReviewContext, CodexRouteDigest, CodexRouteMeaningDigest,
    CodexRouteReviewSummary, CodexUserAction, CodexWordingLintDigest, GateKind, MeaningAtom,
    PatchPlan, ProfileKind, RepoSnapshot, RouteKind, RouteState, WordingLintReport,
    WordingRuleKind, SCHEMA_VERSION, TOOL_NAME,
};
use std::collections::BTreeSet;

#[must_use]
pub fn build_calibration_source_summary(
    run: &CalibrationRun,
) -> CalibrationSourceVisibilitySummary {
    run.source_visibility_summary()
}

#[must_use]
pub fn build_review_context(snapshot: &RepoSnapshot, plan: &PatchPlan) -> CodexReviewContext {
    build_review_context_with_wording(snapshot, plan, None)
}

#[must_use]
pub fn build_review_context_with_wording(
    snapshot: &RepoSnapshot,
    plan: &PatchPlan,
    wording_lint: Option<&WordingLintReport>,
) -> CodexReviewContext {
    let audit = audit_summary(snapshot);
    let findings = snapshot
        .findings
        .iter()
        .filter(|finding| match finding.recommendation.as_ref() {
            Some(recommendation) => recommendation.gate != GateKind::Manual,
            None => true,
        })
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
            safety: operation.safety,
            preview_only: operation.preview_only,
            requires_confirmation: operation.requires_confirmation,
            path: operation.path.clone(),
            title: operation.title.clone(),
            planned_change: operation.planned_change.clone(),
        })
        .collect::<Vec<_>>();
    let blocked_items = plan
        .blocked
        .iter()
        .filter(|item| item.gate == GateKind::Guarded)
        .map(|item| CodexBlockedDigest {
            id: item.id.clone(),
            gate: item.gate,
            source: item.source,
            safety: item.safety,
            route: item.route,
            priority: item.priority,
            pattern_id: item.pattern_id.clone(),
            title: item.title.clone(),
            reason: item.reason.clone(),
        })
        .collect::<Vec<_>>();
    let route_review = route_review_summary(snapshot, plan);
    let claims = claim_summary(snapshot);
    let wording_lint = wording_lint_digest(wording_lint);
    let route_meanings = route_meaning_digests(snapshot);
    let routes = route_digests(snapshot);
    let co_occurrence_gaps = co_occurrence_digests(snapshot);
    let profile = snapshot.profile.as_ref().map(|profile| profile.profile);
    let pr_draft = build_pr_draft(
        snapshot,
        plan,
        &audit,
        &route_review,
        &claims,
        &wording_lint,
        &route_meanings,
    );
    let user_actions = build_user_actions(snapshot, profile);

    CodexReviewContext {
        schema_version: SCHEMA_VERSION.to_string(),
        tool: TOOL_NAME.to_string(),
        repo_root: snapshot.repo_root.clone(),
        profile,
        audit,
        route_review,
        claims,
        wording_lint,
        route_meanings,
        routes,
        co_occurrence_gaps,
        plan: plan.summary,
        findings,
        safe_operations,
        blocked_items,
        user_actions,
        pr_draft,
        calibration_sources: CalibrationSourceVisibilitySummary::default(),
        claim_boundary: "Codex context is a draft review artifact generated from RepoSeiri Rust core outputs. Detailed claim boundaries are referenced by claim id; this context does not create branches, write files, call GitHub, open PRs, adopt policies, or guarantee popularity, trust, security, or quality.".to_string(),
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
    if let (Some(profile), Some(confidence)) = (
        context.audit.top_profile,
        context.audit.top_profile_confidence_x100,
    ) {
        out.push_str(&format!(
            "- Top profile branch: `{profile}` confidence `{confidence}` / `100` across `{}` candidates\n",
            context.audit.profile_branches
        ));
    }
    if let (Some(route), Some(priority)) = (
        context.audit.top_missing_route,
        context.audit.top_missing_route_priority_x100,
    ) {
        out.push_str(&format!(
            "- Top missing route: `{:?}` priority `{priority}` / `100` across `{}` candidates\n",
            route, context.audit.missing_route_priorities
        ));
    }
    out.push_str(&format!(
        "- Co-occurrence gaps: `{}`\n",
        context.audit.co_occurrence_gaps
    ));
    out.push_str(&format!("- Findings: `{}`\n", context.audit.findings));
    out.push_str(&format!(
        "- Route review: strong `{}` / weak `{}` / missing `{}`\n",
        context.route_review.strong_routes,
        context.route_review.weak_routes,
        context.route_review.missing_routes
    ));
    out.push_str(&format!(
        "- Codex actions: safe fixes `{}` / guarded drafts `{}` / manual decisions withheld `{}`\n",
        context.route_review.safe_fixes,
        context.route_review.guarded_drafts,
        context.route_review.manual_decisions
    ));
    out.push_str(&format!(
        "- Evidence ledger records: `{}`\n",
        context.audit.evidence_ledger_records
    ));
    out.push_str(&format!(
        "- Route states: `{}`\n",
        context.audit.route_states
    ));
    out.push_str(&format!(
        "- Content claims: `{}`\n",
        context.audit.content_claims
    ));
    out.push_str(&format!(
        "- Wording lint findings: `{}`\n",
        context.wording_lint.findings
    ));
    out.push_str(&format!(
        "- Route meaning digests: `{}`\n",
        context.route_meanings.len()
    ));
    if context.calibration_sources.total > 0 {
        out.push_str(&format!(
            "- Calibration sources: public `{}` / local_only `{}` / redacted `{}` / pending_review `{}`\n",
            context.calibration_sources.public_sources,
            context.calibration_sources.local_only_sources,
            context.calibration_sources.redacted_sources,
            context.calibration_sources.pending_review
        ));
    }
    out.push_str(&format!(
        "- Safe operations: `{}`\n",
        context.plan.safe_operations
    ));
    out.push_str(&format!(
        "- Guarded drafts: `{}`\n",
        context.blocked_items.len()
    ));
    out.push_str(&format!("- Boundary: {}\n\n", context.claim_boundary));

    out.push_str("## Claim Summary\n\n");
    out.push_str(&format!(
        "- Total `{}` / observed `{}` / inferred `{}` / suggested `{}` / blocked `{}`\n",
        context.claims.total,
        context.claims.observed,
        context.claims.inferred,
        context.claims.suggested,
        context.claims.blocked
    ));
    out.push_str(&format!(
        "- Routes with claims `{}` / evidence-linked claims `{}`\n",
        context.claims.routes_with_claims, context.claims.evidence_linked_claims
    ));
    out.push_str(&format!(
        "- Boundary kinds: {}\n\n",
        debug_values_or_none(&context.claims.boundary_kinds)
    ));

    out.push_str("## Wording Lint\n\n");
    out.push_str(&format!(
        "- Available: `{}`\n",
        context.wording_lint.available
    ));
    out.push_str(&format!(
        "- Files scanned `{}` / generated surfaces `{}` / findings `{}` / suppressed boundary exceptions `{}`\n",
        context.wording_lint.files_scanned,
        context.wording_lint.generated_surfaces,
        context.wording_lint.findings,
        context.wording_lint.suppressed_boundary_exceptions
    ));
    out.push_str(&format!(
        "- Rules: {}\n",
        debug_values_or_none(&context.wording_lint.rules)
    ));
    out.push_str(&format!(
        "- Boundary kinds: {}\n\n",
        debug_values_or_none(&context.wording_lint.boundary_kinds)
    ));

    out.push_str("## Route Meaning Digest\n\n");
    if context.route_meanings.is_empty() {
        out.push_str("- No route meaning digests emitted.\n\n");
    } else {
        for digest in &context.route_meanings {
            out.push_str(&format!(
                "- `{:?}` `{:?}` indicates {} / does_not_indicate {}\n",
                digest.route,
                digest.state,
                debug_values_or_none(&digest.indicates),
                debug_values_or_none(&digest.does_not_indicate)
            ));
        }
        out.push('\n');
    }

    out.push_str("## Route Review\n\n");
    for digest in &context.routes {
        out.push_str(&format!(
            "- `{:?}` `{:?}` confidence `{:?}` priority `{}` gate `{}` claims {} boundary_kinds `{}`: {}\n",
            digest.route,
            digest.state,
            digest.confidence,
            digest
                .priority_score_x100
                .map_or_else(|| "n/a".to_string(), |score| score.to_string()),
            digest
                .gate
                .map_or_else(|| "n/a".to_string(), |gate| format!("{gate:?}")),
            claim_ids_or_none(&digest.claim_ids),
            digest.boundary_kinds.len(),
            digest.reason
        ));
    }
    out.push('\n');

    out.push_str("## Co-occurrence Gaps\n\n");
    if context.co_occurrence_gaps.is_empty() {
        out.push_str("- No co-occurrence gaps emitted.\n\n");
    } else {
        for gap in &context.co_occurrence_gaps {
            out.push_str(&format!(
                "- `{}` `{:?}` gate `{:?}` missing_routes `{:?}` missing_signals `{}`: {}\n",
                gap.id,
                gap.priority,
                gap.gate,
                gap.missing_routes,
                gap.missing_signals.join(", "),
                gap.title
            ));
        }
        out.push('\n');
    }

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

fn claim_summary(snapshot: &RepoSnapshot) -> CodexClaimSummary {
    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    let routes_with_claims = snapshot
        .claims
        .iter()
        .map(|claim| claim.route)
        .collect::<BTreeSet<RouteKind>>()
        .len();
    let evidence_linked_claims = snapshot
        .claims
        .iter()
        .filter(|claim| !claim.evidence_ids.is_empty())
        .count();

    CodexClaimSummary {
        total: snapshot.claims.len(),
        observed: claim_index.strength_count(ClaimStrength::Observed),
        inferred: claim_index.strength_count(ClaimStrength::Inferred),
        suggested: claim_index.strength_count(ClaimStrength::Suggested),
        blocked: claim_index.strength_count(ClaimStrength::Blocked),
        routes_with_claims,
        evidence_linked_claims,
        boundary_kinds: claim_index.boundary_kinds(),
    }
}

fn wording_lint_digest(report: Option<&WordingLintReport>) -> CodexWordingLintDigest {
    let Some(report) = report else {
        return CodexWordingLintDigest::default();
    };
    CodexWordingLintDigest {
        available: true,
        files_scanned: report.summary.files_scanned,
        generated_surfaces: report.summary.generated_surfaces,
        findings: report.summary.findings,
        suppressed_boundary_exceptions: report.summary.suppressed_boundary_exceptions,
        rules: report
            .findings
            .iter()
            .map(|finding| finding.rule)
            .collect::<BTreeSet<WordingRuleKind>>()
            .into_iter()
            .collect(),
        boundary_kinds: report
            .findings
            .iter()
            .map(|finding| finding.boundary)
            .collect::<BTreeSet<ClaimBoundaryKind>>()
            .into_iter()
            .collect(),
    }
}

fn route_meaning_digests(snapshot: &RepoSnapshot) -> Vec<CodexRouteMeaningDigest> {
    snapshot
        .route_states
        .iter()
        .map(|state| {
            let rule = route_meaning_rule(state.route, state.state);
            CodexRouteMeaningDigest {
                route: state.route,
                state: state.state,
                indicates: rule.indicates.to_vec(),
                does_not_indicate: rule.does_not_indicate.to_vec(),
            }
        })
        .collect()
}

fn audit_summary(snapshot: &RepoSnapshot) -> CodexAuditSummary {
    let baseline = snapshot.baseline.as_ref();
    let strong_routes = snapshot
        .route_states
        .iter()
        .filter(|route| route_strength(route.state) == RouteStrength::Strong)
        .count();
    let weak_routes = snapshot
        .route_states
        .iter()
        .filter(|route| route_strength(route.state) == RouteStrength::Weak)
        .count();
    let missing_routes = snapshot
        .route_states
        .iter()
        .filter(|route| route_strength(route.state) == RouteStrength::Missing)
        .count();
    CodexAuditSummary {
        entries_scanned: snapshot.entry_count,
        evidence_items: snapshot.evidence.len(),
        evidence_ledger_records: snapshot.evidence_ledger.len(),
        route_states: snapshot.route_states.len(),
        content_claims: snapshot.claims.len(),
        strong_routes,
        weak_routes,
        missing_routes,
        findings: snapshot.findings.len(),
        pattern_matches: snapshot.pattern_matches.len(),
        profile_score_x100: snapshot
            .profile
            .as_ref()
            .map(|profile| profile.score.score_x100),
        profile_branches: snapshot
            .profile
            .as_ref()
            .map_or(0, |profile| profile.branches.len()),
        top_profile: snapshot
            .profile
            .as_ref()
            .and_then(|profile| profile.branch_summary.top_profile),
        top_profile_confidence_x100: snapshot
            .profile
            .as_ref()
            .and_then(|profile| profile.branch_summary.top_confidence_x100),
        missing_route_priorities: snapshot.missing_route_priority.priorities.len(),
        co_occurrence_gaps: snapshot
            .missing_route_priority
            .co_occurrence_gaps
            .iter()
            .filter(|gap| gap.gate != GateKind::Manual)
            .count(),
        top_missing_route: snapshot.missing_route_priority.summary.top_route,
        top_missing_route_priority_x100: snapshot.missing_route_priority.summary.top_priority_x100,
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
    route_review: &CodexRouteReviewSummary,
    claims: &CodexClaimSummary,
    wording_lint: &CodexWordingLintDigest,
    route_meanings: &[CodexRouteMeaningDigest],
) -> CodexPrDraft {
    let mut body = String::new();
    body.push_str("## Summary\n\n");
    body.push_str("- Generated from RepoSeiri Rust core audit and dry-run plan output.\n");
    body.push_str(&format!(
        "- Scanned `{}` entries and collected `{}` evidence items / `{}` ledger records.\n",
        audit.entries_scanned, audit.evidence_items, audit.evidence_ledger_records
    ));
    body.push_str(&format!(
        "- Route states emitted: `{}`.\n",
        audit.route_states
    ));
    body.push_str(&format!(
        "- Content claims emitted: `{}`; route details reference claim ids where available.\n",
        audit.content_claims
    ));
    body.push_str(&format!(
        "- Claim digest: observed `{}` / inferred `{}` / suggested `{}` / blocked `{}` across `{}` routes.\n",
        claims.observed, claims.inferred, claims.suggested, claims.blocked, claims.routes_with_claims
    ));
    body.push_str(&format!(
        "- Wording lint digest: available `{}` / findings `{}` / suppressed boundary exceptions `{}`.\n",
        wording_lint.available,
        wording_lint.findings,
        wording_lint.suppressed_boundary_exceptions
    ));
    body.push_str(&format!(
        "- Route meaning digests emitted: `{}`.\n",
        route_meanings.len()
    ));
    body.push_str(&format!(
        "- Route review: strong `{}` / weak `{}` / missing `{}`.\n",
        route_review.strong_routes, route_review.weak_routes, route_review.missing_routes
    ));
    if let (Some(route), Some(priority)) = (
        audit.top_missing_route,
        audit.top_missing_route_priority_x100,
    ) {
        body.push_str(&format!(
            "- Top missing route priority: `{:?}` at `{priority}` / `100` across `{}` candidates.\n",
            route, audit.missing_route_priorities
        ));
    }
    body.push_str(&format!(
        "- Co-occurrence gaps emitted: `{}`.\n",
        audit.co_occurrence_gaps
    ));
    body.push_str(&format!("- Findings: `{}`.\n", audit.findings));
    body.push_str(&format!(
        "- Safe fixes `{}` and guarded drafts `{}` are included for Codex review; manual decisions `{}` are withheld from actionable context.\n\n",
        route_review.safe_fixes, route_review.guarded_drafts, route_review.manual_decisions
    ));

    body.push_str("## Route Review\n\n");
    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    for state in &snapshot.route_states {
        if matches!(
            state.state,
            RouteState::Verified
                | RouteState::Structured
                | RouteState::Routed
                | RouteState::Weak
                | RouteState::Absent
                | RouteState::UnsafeToInvent
        ) {
            body.push_str(&format!(
                "- `{:?}` `{:?}` confidence `{:?}` claims {}: {}\n",
                state.route,
                state.state,
                state.confidence,
                claim_ids_or_none(&claim_index.claim_ids_for_route_state(state.route, state.state)),
                state.reason
            ));
        }
    }
    body.push('\n');

    body.push_str("## Claim / Wording / Meaning Digest\n\n");
    body.push_str(&format!(
        "- Claim boundaries: {}\n",
        debug_values_or_none(&claims.boundary_kinds)
    ));
    body.push_str(&format!(
        "- Wording rules: {}\n",
        debug_values_or_none(&wording_lint.rules)
    ));
    let human_review_routes = route_meanings
        .iter()
        .filter(|digest| digest.indicates.contains(&MeaningAtom::HumanReviewRequired))
        .count();
    body.push_str(&format!(
        "- Route meanings requiring human review: `{human_review_routes}`.\n\n"
    ));

    body.push_str("## Safe Fixes\n\n");
    if plan.operations.is_empty() {
        body.push_str("- No safe operations were generated.\n\n");
    } else {
        for operation in &plan.operations {
            body.push_str(&format!(
                "- `{}` `{}` preview-only `{}` confirmation `{}`: {}\n",
                operation.id,
                operation.path,
                operation.preview_only,
                operation.requires_confirmation,
                operation.planned_change
            ));
        }
        body.push('\n');
    }

    body.push_str("## Guarded Drafts\n\n");
    let guarded_items = plan
        .blocked
        .iter()
        .filter(|item| item.gate == GateKind::Guarded)
        .collect::<Vec<_>>();
    if guarded_items.is_empty() {
        body.push_str("- No reviewable guarded drafts are included in this plan.\n\n");
    } else {
        for item in guarded_items {
            let kind = item
                .suggested_kind
                .map_or_else(|| "none".to_string(), |kind| format!("{kind:?}"));
            body.push_str(&format!(
                "- `{}` `{:?}` kind `{}` `{}`: {}\n",
                item.id, item.gate, kind, item.pattern_id, item.reason
            ));
        }
        body.push('\n');
    }

    let manual_decisions = plan
        .blocked
        .iter()
        .filter(|item| item.gate == GateKind::Manual)
        .count();
    body.push_str("## Manual Decisions\n\n");
    body.push_str(&format!(
        "- `{manual_decisions}` manual decisions were withheld from Codex actionable context and must stay with maintainers.\n\n"
    ));

    body.push_str("## Verification\n\n");
    body.push_str("- [ ] Review generated safe operations before applying them.\n");
    body.push_str("- [ ] Confirm guarded drafts with maintainers before writing content.\n");
    body.push_str("- [ ] Keep manual policy, legal, security, and ownership decisions outside automated Codex action.\n");
    body.push_str("- [ ] Run `cargo test --workspace` after any applied changes.\n\n");

    body.push_str("## Boundary\n\n");
    body.push_str("- Claim details and blocked guarantee boundaries are referenced through claim ids in this draft.\n");
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

fn route_review_summary(snapshot: &RepoSnapshot, plan: &PatchPlan) -> CodexRouteReviewSummary {
    CodexRouteReviewSummary {
        strong_routes: snapshot
            .route_states
            .iter()
            .filter(|route| route_strength(route.state) == RouteStrength::Strong)
            .count(),
        weak_routes: snapshot
            .route_states
            .iter()
            .filter(|route| route_strength(route.state) == RouteStrength::Weak)
            .count(),
        missing_routes: snapshot
            .route_states
            .iter()
            .filter(|route| route_strength(route.state) == RouteStrength::Missing)
            .count(),
        co_occurrence_gaps: snapshot.missing_route_priority.co_occurrence_gaps.len(),
        safe_fixes: plan.operations.len(),
        guarded_drafts: plan
            .blocked
            .iter()
            .filter(|item| item.gate == GateKind::Guarded)
            .count(),
        manual_decisions: plan
            .blocked
            .iter()
            .filter(|item| item.gate == GateKind::Manual)
            .count(),
    }
}

fn route_digests(snapshot: &RepoSnapshot) -> Vec<CodexRouteDigest> {
    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    snapshot
        .route_states
        .iter()
        .map(|state| {
            let priority = snapshot
                .missing_route_priority
                .priorities
                .iter()
                .find(|priority| priority.route == state.route);
            CodexRouteDigest {
                route: state.route,
                state: state.state,
                confidence: state.confidence,
                evidence_ids: state.evidence_ids.clone(),
                claim_ids: claim_index.claim_ids_for_route_state(state.route, state.state),
                boundary_kinds: claim_index
                    .boundary_kinds_for_route_state(state.route, state.state),
                priority_score_x100: priority.map(|priority| priority.priority_score_x100),
                gate: priority.and_then(|priority| {
                    if priority.gate == GateKind::Manual {
                        None
                    } else {
                        Some(priority.gate)
                    }
                }),
                reason: state.reason.clone(),
            }
        })
        .collect()
}

fn claim_ids_or_none(ids: &[ClaimId]) -> String {
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.iter()
            .map(|id| format!("`{id}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn debug_values_or_none<T: std::fmt::Debug>(values: &[T]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("`{value:?}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn co_occurrence_digests(snapshot: &RepoSnapshot) -> Vec<CodexCoOccurrenceDigest> {
    snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .iter()
        .filter(|gap| gap.gate != GateKind::Manual)
        .map(|gap| CodexCoOccurrenceDigest {
            id: gap.id.clone(),
            title: gap.title.clone(),
            gate: gap.gate,
            priority: gap.priority,
            present_routes: gap.present_routes.clone(),
            missing_routes: gap.missing_routes.clone(),
            missing_signals: gap.missing_signals.clone(),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteStrength {
    Strong,
    Weak,
    Missing,
}

fn route_strength(state: RouteState) -> RouteStrength {
    match state {
        RouteState::Routed
        | RouteState::Structured
        | RouteState::Verified
        | RouteState::Overridden => RouteStrength::Strong,
        RouteState::Absent | RouteState::UnsafeToInvent => RouteStrength::Missing,
        RouteState::Implicit
        | RouteState::Weak
        | RouteState::Inherited
        | RouteState::Conflicting
        | RouteState::Overloaded
        | RouteState::Stale => RouteStrength::Weak,
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
