use seiri_core::{
    stable_id, BaselineStatus, CalibrationRun, ClaimBoundaryKind, ClaimId, ClaimRefIndex,
    ClaimStrength, CodexReviewContext, Evidence, EvidenceKind, EvidenceSource, ImportantFileKind,
    PatchPlan, ProfileKind, RepoSnapshot, RouteKind, RouteState, WordingLintReport,
};
use seiri_fs::RepoFsScan;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::Path;

mod claims;
mod evidence;
mod route_priority;
mod wording;

use claims::build_content_claims;
use evidence::{build_evidence_ledger, build_route_states};
use route_priority::build_missing_route_priority_report;

#[derive(Debug)]
pub enum AuditError {
    Fs(seiri_fs::FsError),
    Markdown(seiri_markdown::MarkdownError),
    Calibration(seiri_calibration::CalibrationError),
    Json(serde_json::Error),
    Io {
        path: std::path::PathBuf,
        source: io::Error,
    },
}

impl Display for AuditError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fs(error) => write!(f, "{error}"),
            Self::Markdown(error) => write!(f, "{error}"),
            Self::Calibration(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Io { path, source } => write!(f, "failed to read {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for AuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Fs(error) => Some(error),
            Self::Markdown(error) => Some(error),
            Self::Calibration(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<seiri_fs::FsError> for AuditError {
    fn from(value: seiri_fs::FsError) -> Self {
        Self::Fs(value)
    }
}

impl From<seiri_markdown::MarkdownError> for AuditError {
    fn from(value: seiri_markdown::MarkdownError) -> Self {
        Self::Markdown(value)
    }
}

impl From<seiri_calibration::CalibrationError> for AuditError {
    fn from(value: seiri_calibration::CalibrationError) -> Self {
        Self::Calibration(value)
    }
}

impl From<serde_json::Error> for AuditError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn audit_repository(path: impl AsRef<Path>) -> Result<RepoSnapshot, AuditError> {
    audit_repository_with_profile(path, ProfileKind::Common)
}

pub fn audit_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<RepoSnapshot, AuditError> {
    let fs_scan = seiri_fs::scan_repository(path)?;
    let readme = seiri_markdown::analyze_readme(&fs_scan.repo_root)?;
    let repo_root = fs_scan.repo_root.to_string_lossy().replace('\\', "/");
    let mut snapshot = RepoSnapshot::new(repo_root);
    snapshot.entry_count = fs_scan.files.len();
    snapshot.files = fs_scan.files.clone();
    snapshot.important_files = fs_scan.important_files.clone();
    snapshot.readme = readme;
    snapshot.evidence = build_evidence(&fs_scan, snapshot.readme.as_ref());
    snapshot.evidence_ledger = build_evidence_ledger(&snapshot.evidence);
    let baseline = seiri_patterns::evaluate_common_baseline(&snapshot);
    snapshot.pattern_matches = baseline.pattern_matches;
    snapshot.findings = baseline.findings;
    snapshot.baseline = Some(baseline.report);
    snapshot.route_states = build_route_states(
        &snapshot.evidence_ledger,
        &snapshot.pattern_matches,
        snapshot.readme.as_ref(),
    );
    snapshot.profile = seiri_profiles::evaluate_profile(&snapshot, profile);
    snapshot.missing_route_priority = build_missing_route_priority_report(&snapshot);
    snapshot.claims = build_content_claims(&snapshot);
    Ok(snapshot)
}

pub fn to_json(snapshot: &RepoSnapshot) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(snapshot)?)
}

pub fn pattern_registry_to_json() -> Result<String, AuditError> {
    Ok(seiri_patterns::common_registry_to_json()?)
}

#[must_use]
pub fn pattern_registry_to_markdown() -> String {
    seiri_patterns::render_common_registry_markdown()
}

pub fn plan_repository(path: impl AsRef<Path>) -> Result<PatchPlan, AuditError> {
    plan_repository_with_profile(path, ProfileKind::Common)
}

pub fn plan_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<PatchPlan, AuditError> {
    let snapshot = audit_repository_with_profile(path, profile)?;
    Ok(seiri_planner::plan_safe_patches(&snapshot))
}

pub fn plan_to_json(plan: &PatchPlan) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(plan)?)
}

pub fn lint_wording_repository(path: impl AsRef<Path>) -> Result<WordingLintReport, AuditError> {
    lint_wording_repository_with_profile(path, ProfileKind::Common)
}

pub fn lint_wording_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<WordingLintReport, AuditError> {
    wording::lint_repository_with_profile(path, profile)
}

pub fn wording_lint_to_json(report: &WordingLintReport) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(report)?)
}

#[must_use]
pub fn wording_lint_to_markdown(report: &WordingLintReport) -> String {
    wording::render_markdown(report)
}

pub fn calibrate_dataset_path(path: impl AsRef<Path>) -> Result<CalibrationRun, AuditError> {
    let dataset = seiri_calibration::load_dataset(path)?;
    Ok(seiri_calibration::calibrate_dataset(&dataset))
}

pub fn calibration_to_json(run: &CalibrationRun) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(
        &run.redacted_for_public_output(),
    )?)
}

pub fn codex_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<CodexReviewContext, AuditError> {
    let path = path.as_ref();
    let snapshot = audit_repository_with_profile(path, profile)?;
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    let wording_lint = wording::lint_repository_with_profile(path, profile)?;
    Ok(seiri_codex::build_review_context_with_wording(
        &snapshot,
        &plan,
        Some(&wording_lint),
    ))
}

pub fn codex_to_json(context: &CodexReviewContext) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(context)?)
}

pub fn codex_pr_draft_to_json(context: &CodexReviewContext) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(&context.pr_draft)?)
}

#[must_use]
pub fn codex_to_markdown(context: &CodexReviewContext) -> String {
    seiri_codex::render_review_context_markdown(context)
}

#[must_use]
pub fn codex_pr_body_to_markdown(context: &CodexReviewContext) -> String {
    seiri_codex::render_pr_body(context)
}

#[must_use]
pub fn calibration_to_markdown(run: &CalibrationRun) -> String {
    let source_visibility = run.source_visibility_summary();
    let run = run.redacted_for_public_output();
    let mut out = String::new();
    out.push_str("# RepoSeiri Calibration Report\n\n");
    out.push_str(&format!("- Schema: `{}`\n", run.schema_version));
    out.push_str(&format!("- Dataset: `{}`\n", run.dataset_id));
    out.push_str(&format!("- Records: `{}`\n", run.summary.records));
    out.push_str(&format!("- Sources: `{}`\n", run.summary.sources));
    out.push_str(&format!(
        "- Source visibility: public `{}` / local_only `{}` / redacted `{}`\n",
        source_visibility.public_sources,
        source_visibility.local_only_sources,
        source_visibility.redacted_sources
    ));
    out.push_str(&format!(
        "- Source review status: pending_review `{}` / adopted `{}` / deferred `{}` / rejected `{}`\n",
        source_visibility.pending_review,
        source_visibility.adopted,
        source_visibility.deferred,
        source_visibility.rejected
    ));
    out.push_str(&format!(
        "- Known pattern stats: `{}`\n",
        run.summary.known_pattern_stats
    ));
    out.push_str(&format!(
        "- Route requirements: `{}`\n",
        run.summary.route_requirements
    ));
    out.push_str(&format!(
        "- Profile branches: `{}`\n",
        run.summary.profile_branches
    ));
    out.push_str(&format!(
        "- Pending patterns: `{}`\n",
        run.summary.pending_patterns
    ));
    out.push_str(&format!(
        "- Weight suggestions: `{}`\n",
        run.summary.weight_suggestions
    ));
    out.push_str(&format!("- Boundary: {}\n", run.claim_boundary.summary));
    out.push_str(&format!(
        "- Boundary gates: review_required `{}` runtime_rule_adoption `{}` automatic_weight_adoption `{}` guarantees `{}`\n\n",
        run.claim_boundary.review_required,
        run.claim_boundary.runtime_rule_adoption_allowed,
        run.claim_boundary.automatic_weight_adoption_allowed,
        run.claim_boundary.guarantee_allowed
    ));

    out.push_str("## Calibration Sources\n\n");
    if run.sources.is_empty() {
        out.push_str("- No calibration sources recorded.\n\n");
    } else {
        for source in &run.sources {
            out.push_str(&format!(
                "- `{}` kind `{:?}` visibility `{:?}` label `{}` records `{}` scale `{:?}` status `{:?}`\n",
                source.id,
                source.kind,
                source.visibility,
                source.label,
                source.records,
                source.scale,
                source.review_status
            ));
        }
        out.push('\n');
    }

    out.push_str("## Pattern Stats\n\n");
    if run.stats.is_empty() {
        out.push_str("- No known pattern stats generated.\n\n");
    } else {
        for stat in &run.stats {
            out.push_str(&format!("### `{}`\n\n", stat.pattern_id));
            match stat.route {
                Some(route) => out.push_str(&format!("- Route: `{:?}`\n", route)),
                None => out.push_str("- Route: none\n"),
            }
            out.push_str(&format!("- Repositories: `{}`\n", stat.repositories));
            out.push_str(&format!("- Observations: `{}`\n", stat.observations));
            out.push_str(&format!("- Frequency x1000: `{}`\n", stat.frequency_x1000));
            out.push_str(&format!("- Sources: `{}`\n", stat.source_ids.join("`, `")));
            out.push_str(&format!("- Confidence: `{:?}`\n", stat.confidence));
            out.push_str(&format!("- Review status: `{:?}`\n", stat.review_status));
            out.push_str(&format!("- Note: {}\n\n", stat.confidence_note));
        }
    }

    out.push_str("## Route Requirements\n\n");
    if run.route_requirements.is_empty() {
        out.push_str("- No route requirement candidates generated.\n\n");
    } else {
        for requirement in &run.route_requirements {
            out.push_str(&format!(
                "- `{}` route `{:?}` repositories `{}` frequency `{}` requirement `{:?}` priority `{:?}` confidence `{:?}` status `{:?}`\n",
                requirement.id,
                requirement.route,
                requirement.supporting_repositories,
                requirement.frequency_x1000,
                requirement.suggested_requirement,
                requirement.priority,
                requirement.confidence,
                requirement.review_status
            ));
        }
        out.push('\n');
    }

    out.push_str("## Profile Branches\n\n");
    if run.profile_branches.is_empty() {
        out.push_str("- No profile branch candidates generated.\n\n");
    } else {
        for branch in &run.profile_branches {
            out.push_str(&format!(
                "- rank `{}` profile `{}` prior `{}` confidence `{}` score `{}`\n",
                branch.rank,
                branch.profile,
                branch.prior_x1000,
                branch.confidence_x100,
                branch.score_x100
            ));
        }
        out.push('\n');
    }

    out.push_str("## Pending Pattern Candidates\n\n");
    if run.pending_patterns.is_empty() {
        out.push_str("- No pending pattern candidates.\n\n");
    } else {
        for candidate in &run.pending_patterns {
            out.push_str(&format!(
                "- `{}` `{}` repositories `{}` observations `{}` status `{:?}`\n",
                candidate.id,
                candidate.raw_label,
                candidate.observed_repositories,
                candidate.observations,
                candidate.review_status
            ));
        }
        out.push('\n');
    }

    out.push_str("## Weight Suggestions\n\n");
    if run.weight_suggestions.is_empty() {
        out.push_str("- No weight suggestions generated.\n");
    } else {
        for suggestion in &run.weight_suggestions {
            let current = suggestion
                .current_weight
                .map_or_else(|| "none".to_string(), |weight| weight.to_string());
            let route = suggestion
                .route
                .map_or_else(|| "none".to_string(), |route| format!("{route:?}"));
            out.push_str(&format!(
                "- `{}` `{}` route `{}` profile `{}` current `{}` suggested `{}` delta `{}` confidence `{:?}` status `{:?}`\n",
                suggestion.id,
                suggestion.pattern_id,
                route,
                suggestion.profile,
                current,
                suggestion.suggested_weight,
                suggestion.suggested_delta,
                suggestion.confidence,
                suggestion.review_status
            ));
        }
    }

    out
}

#[must_use]
pub fn plan_to_markdown(plan: &PatchPlan) -> String {
    let mut out = String::new();
    out.push_str("# RepoSeiri Patch Plan\n\n");
    out.push_str(&format!("- Schema: `{}`\n", plan.schema_version));
    out.push_str(&format!("- Planner: `{}`\n", plan.planner_version));
    out.push_str(&format!("- Mode: `{:?}`\n", plan.mode));
    match plan.profile {
        Some(profile) => out.push_str(&format!("- Profile: `{profile}`\n")),
        None => out.push_str("- Profile: not selected\n"),
    }
    out.push_str(&format!(
        "- Total candidates: `{}`\n",
        plan.summary.total_candidates
    ));
    out.push_str(&format!(
        "- Safe operations: `{}`\n",
        plan.summary.safe_operations
    ));
    out.push_str(&format!(
        "- Safe blocked: `{}`\n",
        plan.summary.safe_blocked
    ));
    out.push_str(&format!(
        "- Guarded items: `{}`\n",
        plan.summary.guarded_items
    ));
    out.push_str(&format!(
        "- Manual items: `{}`\n",
        plan.summary.manual_items
    ));
    out.push_str(&format!(
        "- Preview-only operations: `{}`\n",
        plan.summary.preview_only_operations
    ));
    out.push_str(&format!(
        "- Preflight passed: `{}`\n",
        plan.summary.preflight_passed
    ));
    out.push_str(&format!(
        "- Preflight failed or blocked: `{}`\n",
        plan.summary.preflight_failed
    ));
    out.push_str(&format!(
        "- Safety policy: writes_files `{}` applies_patches `{}` safe_gate_only `{}` existing_targets `{}` unsafe_to_invent_blocked `{}`\n",
        plan.safety_policy.writes_files,
        plan.safety_policy.applies_patches,
        plan.safety_policy.safe_gate_only,
        plan.safety_policy.requires_existing_targets,
        plan.safety_policy.blocks_unsafe_to_invent
    ));
    out.push_str(&format!("- Boundary: {}\n\n", plan.claim_boundary));

    out.push_str("## Safe Fixes\n\n");
    if plan.operations.is_empty() {
        out.push_str("- No safe fixes generated.\n\n");
    } else {
        for operation in &plan.operations {
            out.push_str(&format!("### {}\n\n", operation.id));
            out.push_str(&format!("- Gate: `{:?}`\n", operation.gate));
            out.push_str(&format!("- Kind: `{:?}`\n", operation.kind));
            out.push_str(&format!("- Source: `{:?}`\n", operation.source));
            out.push_str(&format!("- Safety: `{:?}`\n", operation.safety));
            out.push_str(&format!("- Priority: `{:?}`\n", operation.priority));
            out.push_str(&format!("- Preview only: `{}`\n", operation.preview_only));
            out.push_str(&format!(
                "- Requires confirmation: `{}`\n",
                operation.requires_confirmation
            ));
            out.push_str(&format!("- Path: `{}`\n", operation.path));
            out.push_str(&format!("- Pattern: `{}`\n", operation.pattern_id));
            if let Some(finding_id) = &operation.finding_id {
                out.push_str(&format!("- Finding: `{finding_id}`\n"));
            }
            out.push_str(&format!("- Change: {}\n", operation.planned_change));
            out.push_str(&format!("- Rationale: {}\n\n", operation.rationale));
            out.push_str("Preflight:\n");
            for check in &operation.preflight {
                out.push_str(&format!(
                    "- `{:?}` `{:?}`: {}\n",
                    check.kind, check.status, check.detail
                ));
            }
            out.push('\n');
            out.push_str("```diff\n");
            for line in &operation.diff_preview {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
    }

    out.push_str("## Guarded Drafts\n\n");
    let guarded_items = plan
        .blocked
        .iter()
        .filter(|item| item.gate == seiri_core::GateKind::Guarded)
        .collect::<Vec<_>>();
    if guarded_items.is_empty() {
        out.push_str("- No guarded drafts.\n\n");
    } else {
        for item in guarded_items {
            out.push_str(&format!(
                "- `{}` `{:?}` `{:?}` `{:?}` `{}`: {}\n",
                item.id, item.gate, item.source, item.safety, item.pattern_id, item.reason
            ));
            if let Some(kind) = item.suggested_kind {
                out.push_str(&format!("  Suggested kind: `{:?}`\n", kind));
            }
            if let Some(route) = item.route {
                out.push_str(&format!("  Route: `{:?}`\n", route));
            }
            for check in &item.preflight {
                out.push_str(&format!(
                    "  Preflight `{:?}` `{:?}`: {}\n",
                    check.kind, check.status, check.detail
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("## Manual Decisions\n\n");
    let manual_items = plan
        .blocked
        .iter()
        .filter(|item| item.gate == seiri_core::GateKind::Manual)
        .collect::<Vec<_>>();
    if manual_items.is_empty() {
        out.push_str("- No manual decisions.\n");
    } else {
        for item in manual_items {
            out.push_str(&format!(
                "- `{}` `{:?}` `{:?}` `{}`: {}\n",
                item.id, item.source, item.safety, item.pattern_id, item.reason
            ));
            if let Some(kind) = item.suggested_kind {
                out.push_str(&format!("  Suggested kind: `{:?}`\n", kind));
            }
            if let Some(route) = item.route {
                out.push_str(&format!("  Route: `{:?}`\n", route));
            }
        }
    }

    out
}

#[must_use]
pub fn to_markdown(snapshot: &RepoSnapshot) -> String {
    let mut out = String::new();
    out.push_str("# RepoSeiri Report\n\n");
    out.push_str(&format!("- Schema: `{}`\n", snapshot.schema_version));
    out.push_str(&format!("- Repository: `{}`\n", snapshot.repo_root));
    out.push_str(&format!("- Entries scanned: `{}`\n", snapshot.entry_count));
    out.push_str(&format!(
        "- Pattern matches: `{}`\n",
        snapshot.pattern_matches.len()
    ));
    if let Some(baseline) = &snapshot.baseline {
        out.push_str(&format!(
            "- Baseline required: `{}` present / `{}` missing\n",
            baseline.summary.required_present, baseline.summary.required_missing
        ));
        out.push_str(&format!(
            "- Baseline optional: `{}` present / `{}` missing\n",
            baseline.summary.optional_present, baseline.summary.optional_missing
        ));
    }
    if let Some(profile) = &snapshot.profile {
        out.push_str(&format!(
            "- Profile score: `{}` / `100` for `{}`\n",
            profile.score.score_x100, profile.profile
        ));
        if let (Some(top_profile), Some(confidence)) = (
            profile.branch_summary.top_profile,
            profile.branch_summary.top_confidence_x100,
        ) {
            out.push_str(&format!(
                "- Profile branch top: `{}` confidence `{}` / `100` across `{}` candidates\n",
                top_profile, confidence, profile.branch_summary.emitted_profiles
            ));
        }
    }
    out.push_str(&format!(
        "- Evidence items: `{}`\n",
        snapshot.evidence.len()
    ));
    out.push_str(&format!(
        "- Evidence ledger records: `{}`\n",
        snapshot.evidence_ledger.len()
    ));
    out.push_str(&format!("- Content claims: `{}`\n", snapshot.claims.len()));
    out.push_str(&format!(
        "- Route states: `{}`\n",
        snapshot.route_states.len()
    ));
    let (strong_routes, weak_routes, missing_routes) = route_strength_counts(snapshot);
    out.push_str(&format!(
        "- Route review: strong `{strong_routes}` / weak `{weak_routes}` / missing `{missing_routes}`\n"
    ));
    if let (Some(route), Some(priority)) = (
        snapshot.missing_route_priority.summary.top_route,
        snapshot.missing_route_priority.summary.top_priority_x100,
    ) {
        out.push_str(&format!(
            "- Missing route top: `{:?}` priority `{}` / `100` across `{}` candidates\n",
            route, priority, snapshot.missing_route_priority.summary.candidates
        ));
    }
    out.push_str(&format!(
        "- Co-occurrence gaps: `{}`\n",
        snapshot.missing_route_priority.summary.co_occurrence_gaps
    ));
    out.push_str(&format!("- Findings: `{}`\n\n", snapshot.findings.len()));

    out.push_str("## Route Review v2\n\n");
    if snapshot.route_states.is_empty() {
        out.push_str("- No route states emitted.\n\n");
    } else {
        out.push_str("### Strong Routes\n\n");
        render_route_strength_group(&mut out, snapshot, RouteStrength::Strong);
        out.push_str("### Weak Routes\n\n");
        render_route_strength_group(&mut out, snapshot, RouteStrength::Weak);
        out.push_str("### Missing Routes\n\n");
        render_route_strength_group(&mut out, snapshot, RouteStrength::Missing);
    }
    out.push_str(&format!(
        "### Decision Gates\n\n- Missing route priorities: safe `{}` / guarded `{}` / manual `{}`\n\n",
        snapshot.missing_route_priority.summary.safe_gated,
        snapshot.missing_route_priority.summary.guarded_gated,
        snapshot.missing_route_priority.summary.manual_gated
    ));

    render_content_claims(&mut out, snapshot);

    out.push_str("## README\n\n");
    match &snapshot.readme {
        Some(readme) => {
            out.push_str(&format!("- Path: `{}`\n", readme.path));
            out.push_str(&format!("- Headings: `{}`\n", readme.headings.len()));
            out.push_str(&format!("- Links: `{}`\n", readme.links.len()));
            out.push_str(&format!("- Badges: `{}`\n", readme.badges.len()));
            out.push_str(&format!(
                "- Route candidates: `{}`\n\n",
                readme.route_candidates.len()
            ));
            out.push_str(&format!(
                "- Route map: `{}` routed / `{}` weak / `{}` conflicting / `{}` overloaded / `{}` stale / `{}` absent\n\n",
                readme.route_map.summary.routed,
                readme.route_map.summary.weak,
                readme.route_map.summary.conflicting,
                readme.route_map.summary.overloaded,
                readme.route_map.summary.stale,
                readme.route_map.summary.absent
            ));
        }
        None => out.push_str("- Path: not found\n\n"),
    }

    out.push_str("## README Route Map\n\n");
    match &snapshot.readme {
        Some(readme) => {
            for entry in &readme.route_map.entries {
                let gap = entry
                    .observed_gap_count
                    .map_or_else(|| "n/a".to_string(), |count| count.to_string());
                let targets = if entry.targets.is_empty() {
                    "none".to_string()
                } else {
                    entry
                        .targets
                        .iter()
                        .map(|target| {
                            format!(
                                "{} ({:?}, line {})",
                                target.target, target.status, target.line
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                out.push_str(&format!(
                    "- `{:?}` `{:?}` candidates `{}` targets `{}` stale `{}` conflicts `{}` gap `{}`: {}\n",
                    entry.route,
                    entry.state,
                    entry.candidate_count,
                    entry.target_count,
                    entry.stale_target_count,
                    entry.conflicting_target_count,
                    gap,
                    entry.reason
                ));
                out.push_str(&format!("  Targets: {targets}\n"));
            }
            out.push('\n');
        }
        None => out.push_str("- README was not found, so no route map was emitted.\n\n"),
    }

    out.push_str("## Important Files\n\n");
    if snapshot.important_files.is_empty() {
        out.push_str("- None detected\n\n");
    } else {
        for important in &snapshot.important_files {
            out.push_str(&format!("- `{:?}`: `{}`\n", important.kind, important.path));
        }
        out.push('\n');
    }

    out.push_str("## Common Baseline\n\n");
    match &snapshot.baseline {
        Some(baseline) => {
            out.push_str(&format!("- Profile: `{:?}`\n", baseline.profile));
            out.push_str(&format!(
                "- Required present: `{}`\n",
                baseline.summary.required_present
            ));
            out.push_str(&format!(
                "- Required missing: `{}`\n",
                baseline.summary.required_missing
            ));
            out.push_str(&format!(
                "- Optional present: `{}`\n",
                baseline.summary.optional_present
            ));
            out.push_str(&format!(
                "- Optional missing: `{}`\n\n",
                baseline.summary.optional_missing
            ));
            for rule in &baseline.rules {
                let status_marker = match rule.status {
                    BaselineStatus::Present => "present",
                    BaselineStatus::Missing => "missing",
                };
                out.push_str(&format!(
                    "- `{}` `{}` {:?}: {}\n",
                    rule.rule_id, status_marker, rule.requirement, rule.title
                ));
            }
            out.push('\n');
        }
        None => out.push_str("- Baseline was not evaluated.\n\n"),
    }

    out.push_str("## Route States\n\n");
    if snapshot.route_states.is_empty() {
        out.push_str("- No route states emitted.\n\n");
    } else {
        let claim_index = ClaimRefIndex::new(&snapshot.claims);
        for state in &snapshot.route_states {
            let evidence = if state.evidence_ids.is_empty() {
                "none".to_string()
            } else {
                state.evidence_ids.join("`, `")
            };
            let claim_ids = claim_index.claim_ids_for_route_state(state.route, state.state);
            out.push_str(&format!(
                "- `{:?}` `{:?}` confidence `{:?}` evidence `{}` claims {}: {}\n",
                state.route,
                state.state,
                state.confidence,
                evidence,
                claim_ids_or_none(&claim_ids),
                state.reason
            ));
        }
        out.push('\n');
    }

    out.push_str("## Missing Route Priority\n\n");
    out.push_str(&format!(
        "- Boundary: {}\n",
        snapshot.missing_route_priority.boundary
    ));
    out.push_str(&format!(
        "- Gates: safe `{}` / guarded `{}` / manual `{}`\n\n",
        snapshot.missing_route_priority.summary.safe_gated,
        snapshot.missing_route_priority.summary.guarded_gated,
        snapshot.missing_route_priority.summary.manual_gated
    ));
    if snapshot.missing_route_priority.priorities.is_empty() {
        out.push_str("- No missing or degraded route priorities emitted.\n\n");
    } else {
        out.push_str("### Route Priorities\n\n");
        let claim_index = ClaimRefIndex::new(&snapshot.claims);
        for priority in &snapshot.missing_route_priority.priorities {
            let score = decimal_confidence(priority.priority_score_x100);
            let observed = priority.observed_missing_repositories.map_or_else(
                || "n/a".to_string(),
                |repositories| repositories.to_string(),
            );
            let baseline = list_or_none(&priority.baseline_pattern_ids);
            let candidate = list_or_none(&priority.candidate_pattern_ids);
            let gaps = list_or_none(&priority.co_occurrence_gap_ids);
            let claim_ids = claim_index.claim_ids_for_route(priority.route);
            let boundary_kinds = claim_index.boundary_kinds_for_route(priority.route);
            out.push_str(&format!(
                "{}. `{:?}` `{:?}` priority `{}` gate `{:?}` observed_missing `{}`: {}\n",
                priority.rank,
                priority.route,
                priority.priority,
                score,
                priority.gate,
                observed,
                priority.reason
            ));
            out.push_str(&format!("   Baseline: {baseline}\n"));
            out.push_str(&format!("   Candidates: {candidate}\n"));
            out.push_str(&format!("   Co-occurrence: {gaps}\n"));
            out.push_str(&format!(
                "   Claim IDs: {}\n",
                claim_ids_or_none(&claim_ids)
            ));
            out.push_str(&format!(
                "   Boundary kinds: {}\n",
                boundary_kinds_or_none(&boundary_kinds)
            ));
        }
        out.push('\n');
    }
    if snapshot
        .missing_route_priority
        .co_occurrence_gaps
        .is_empty()
    {
        out.push_str("### Co-occurrence Gaps\n\n- No co-occurrence gaps emitted.\n\n");
    } else {
        out.push_str("### Co-occurrence Gaps\n\n");
        for gap in &snapshot.missing_route_priority.co_occurrence_gaps {
            let support = decimal_prior(gap.support_x1000);
            let present_routes = routes_or_none(&gap.present_routes);
            let missing_routes = routes_or_none(&gap.missing_routes);
            let present_signals = list_or_none(&gap.present_signals);
            let missing_signals = list_or_none(&gap.missing_signals);
            out.push_str(&format!(
                "- `{}` {:?} support `{}` repos `{}` gate `{:?}`: {}\n",
                gap.id, gap.priority, support, gap.observed_repositories, gap.gate, gap.title
            ));
            out.push_str(&format!("  Present routes: {present_routes}\n"));
            out.push_str(&format!("  Missing routes: {missing_routes}\n"));
            out.push_str(&format!("  Present signals: {present_signals}\n"));
            out.push_str(&format!("  Missing signals: {missing_signals}\n"));
            out.push_str(&format!("  Reason: {}\n", gap.reason));
        }
        out.push('\n');
    }

    out.push_str("## Profile\n\n");
    match &snapshot.profile {
        Some(profile) => {
            out.push_str(&format!("- Selected profile: `{}`\n", profile.profile));
            out.push_str(&format!(
                "- Score view: `{}` / `100`\n",
                profile.score.score_x100
            ));
            out.push_str(&format!(
                "- Weight: `{}` earned / `{}` total\n",
                profile.score.earned_weight, profile.score.total_weight
            ));
            out.push_str(&format!(
                "- Rules: `{}` present / `{}` missing\n",
                profile.score.present_rules, profile.score.missing_rules
            ));
            out.push_str(&format!("- Note: {}\n\n", profile.score.note));
            out.push_str("### Profile Branch Confidence\n\n");
            out.push_str(&format!(
                "- Ambiguous: `{}`\n",
                profile.branch_summary.ambiguous
            ));
            out.push_str(&format!(
                "- Boundary: {}\n\n",
                profile.branch_summary.boundary
            ));
            for branch in &profile.branches {
                let confidence = decimal_confidence(branch.confidence_x100);
                let prior = decimal_prior(branch.prior_x1000);
                let matched = if branch.matched_signals.is_empty() {
                    "none".to_string()
                } else {
                    branch.matched_signals.join("; ")
                };
                out.push_str(&format!(
                    "{}. `{}` confidence `{}` prior `{}` evidence `{}` score `{}`: {}\n",
                    branch.rank,
                    branch.profile,
                    confidence,
                    prior,
                    branch.evidence_score_x100,
                    branch.score_x100,
                    matched
                ));
            }
            out.push('\n');
            if profile.recommendations.is_empty() {
                out.push_str("- No profile recommendations.\n\n");
            } else {
                out.push_str("### Profile Recommendation Order\n\n");
                for recommendation in &profile.recommendations {
                    out.push_str(&format!(
                        "{}. `{}` {:?} weight `{}`: {}\n",
                        recommendation.rank,
                        recommendation.pattern_id,
                        recommendation.priority,
                        recommendation.weight,
                        recommendation.title
                    ));
                }
                out.push('\n');
            }
        }
        None => out.push_str("- Profile was not evaluated.\n\n"),
    }

    out.push_str("## Pattern Matches\n\n");
    if snapshot.pattern_matches.is_empty() {
        out.push_str("- No pattern matches emitted.\n\n");
    } else {
        for pattern_match in &snapshot.pattern_matches {
            out.push_str(&format!(
                "- `{}` {:?}: {}\n",
                pattern_match.pattern_id, pattern_match.outcome, pattern_match.title
            ));
        }
        out.push('\n');
    }

    out.push_str("## Findings\n\n");
    if snapshot.findings.is_empty() {
        out.push_str("- No findings emitted by the common baseline.\n");
    } else {
        for finding in &snapshot.findings {
            out.push_str(&format!("### {}\n\n", finding.id));
            out.push_str(&format!("- Severity: `{:?}`\n", finding.severity));
            out.push_str(&format!("- Title: {}\n", finding.title));
            out.push_str(&format!("- Message: {}\n", finding.message));
            if !finding.evidence_ids.is_empty() {
                out.push_str(&format!(
                    "- Evidence: `{}`\n",
                    finding.evidence_ids.join("`, `")
                ));
            }
            if let Some(recommendation) = &finding.recommendation {
                out.push_str(&format!("- Gate: `{:?}`\n", recommendation.gate));
                out.push_str(&format!("- Recommendation: {}\n", recommendation.message));
            }
            out.push('\n');
        }
    }

    out
}

fn build_evidence(
    fs_scan: &RepoFsScan,
    readme: Option<&seiri_core::ReadmeSummary>,
) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    for important in &fs_scan.important_files {
        evidence.push(Evidence {
            id: stable_id("ev-important-file", evidence.len() + 1),
            kind: EvidenceKind::ImportantFile,
            path: Some(important.path.clone()),
            route: route_for_important_file(important.kind),
            value: format!("{:?}", important.kind),
            source: EvidenceSource {
                scanner: "seiri-fs".to_string(),
                detail: "important file detection".to_string(),
            },
        });
    }

    match readme {
        Some(summary) => {
            evidence.push(Evidence {
                id: stable_id("ev-readme-present", evidence.len() + 1),
                kind: EvidenceKind::ReadmePresent,
                path: Some(summary.path.clone()),
                route: Some(RouteKind::Identity),
                value: "README detected".to_string(),
                source: EvidenceSource {
                    scanner: "seiri-markdown".to_string(),
                    detail: "readme discovery".to_string(),
                },
            });

            for heading in &summary.headings {
                evidence.push(Evidence {
                    id: stable_id("ev-heading", evidence.len() + 1),
                    kind: EvidenceKind::MarkdownHeading,
                    path: Some(summary.path.clone()),
                    route: seiri_markdown::classify_route(&heading.text, None)
                        .ne(&RouteKind::Unknown)
                        .then(|| seiri_markdown::classify_route(&heading.text, None)),
                    value: heading.text.clone(),
                    source: EvidenceSource {
                        scanner: "seiri-markdown".to_string(),
                        detail: format!("heading line {}", heading.line),
                    },
                });
            }

            for link in &summary.links {
                evidence.push(Evidence {
                    id: stable_id("ev-link", evidence.len() + 1),
                    kind: EvidenceKind::MarkdownLink,
                    path: Some(summary.path.clone()),
                    route: link.route,
                    value: format!("{} -> {}", link.text, link.target),
                    source: EvidenceSource {
                        scanner: "seiri-markdown".to_string(),
                        detail: format!("link line {}", link.line),
                    },
                });
            }

            for badge in &summary.badges {
                evidence.push(Evidence {
                    id: stable_id("ev-badge", evidence.len() + 1),
                    kind: EvidenceKind::MarkdownBadge,
                    path: Some(summary.path.clone()),
                    route: Some(RouteKind::Automation),
                    value: format!("{} -> {}", badge.alt, badge.target),
                    source: EvidenceSource {
                        scanner: "seiri-markdown".to_string(),
                        detail: format!("badge line {}", badge.line),
                    },
                });
            }

            for route in &summary.route_candidates {
                evidence.push(Evidence {
                    id: stable_id("ev-route", evidence.len() + 1),
                    kind: EvidenceKind::RouteCandidate,
                    path: Some(summary.path.clone()),
                    route: Some(route.route),
                    value: route.target.as_ref().map_or_else(
                        || route.text.clone(),
                        |target| format!("{} -> {}", route.text, target),
                    ),
                    source: EvidenceSource {
                        scanner: "seiri-markdown".to_string(),
                        detail: format!("{:?} line {}", route.source, route.line),
                    },
                });
            }
        }
        None => evidence.push(Evidence {
            id: stable_id("ev-readme-missing", evidence.len() + 1),
            kind: EvidenceKind::ReadmeMissing,
            path: None,
            route: Some(RouteKind::Identity),
            value: "README not detected".to_string(),
            source: EvidenceSource {
                scanner: "seiri-markdown".to_string(),
                detail: "readme discovery".to_string(),
            },
        }),
    }

    evidence
}

fn route_for_important_file(kind: ImportantFileKind) -> Option<RouteKind> {
    match kind {
        ImportantFileKind::Readme => Some(RouteKind::Identity),
        ImportantFileKind::License => Some(RouteKind::License),
        ImportantFileKind::Contributing => Some(RouteKind::Contributing),
        ImportantFileKind::Security => Some(RouteKind::Security),
        ImportantFileKind::Support => Some(RouteKind::Support),
        ImportantFileKind::IssueTemplate
        | ImportantFileKind::IssueForm
        | ImportantFileKind::PullRequestTemplate => Some(RouteKind::Intake),
        ImportantFileKind::Changelog => Some(RouteKind::Release),
        ImportantFileKind::Codeowners => Some(RouteKind::Ownership),
        ImportantFileKind::CargoToml => Some(RouteKind::Identity),
        ImportantFileKind::DocsDirectory => Some(RouteKind::Docs),
        ImportantFileKind::Workflow => Some(RouteKind::Automation),
        ImportantFileKind::DependencyBot | ImportantFileKind::SecurityAutomation => {
            Some(RouteKind::Automation)
        }
        ImportantFileKind::Gitignore
        | ImportantFileKind::Gitattributes
        | ImportantFileKind::EditorConfig => Some(RouteKind::Hygiene),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteStrength {
    Strong,
    Weak,
    Missing,
}

fn route_strength_counts(snapshot: &RepoSnapshot) -> (usize, usize, usize) {
    let strong = snapshot
        .route_states
        .iter()
        .filter(|state| route_strength(state.state) == RouteStrength::Strong)
        .count();
    let weak = snapshot
        .route_states
        .iter()
        .filter(|state| route_strength(state.state) == RouteStrength::Weak)
        .count();
    let missing = snapshot
        .route_states
        .iter()
        .filter(|state| route_strength(state.state) == RouteStrength::Missing)
        .count();
    (strong, weak, missing)
}

fn render_route_strength_group(out: &mut String, snapshot: &RepoSnapshot, strength: RouteStrength) {
    let mut emitted = 0;
    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    for state in &snapshot.route_states {
        if route_strength(state.state) != strength {
            continue;
        }
        emitted += 1;
        let priority = snapshot
            .missing_route_priority
            .priorities
            .iter()
            .find(|priority| priority.route == state.route);
        let score = priority.map_or_else(
            || "n/a".to_string(),
            |priority| priority.priority_score_x100.to_string(),
        );
        let gate = priority.map_or_else(
            || "n/a".to_string(),
            |priority| format!("{:?}", priority.gate),
        );
        let claim_ids = claim_index.claim_ids_for_route_state(state.route, state.state);
        let boundary_kinds = claim_index.boundary_kinds_for_claim_ids(&claim_ids);
        out.push_str(&format!(
            "- `{:?}` `{:?}` confidence `{:?}` priority `{}` gate `{}` claims {}: {}\n",
            state.route,
            state.state,
            state.confidence,
            score,
            gate,
            claim_ids_or_none(&claim_ids),
            state.reason
        ));
        out.push_str(&format!(
            "  Boundary kinds: {}\n",
            boundary_kinds_or_none(&boundary_kinds)
        ));
    }
    if emitted == 0 {
        out.push_str("- None.\n");
    }
    out.push('\n');
}

fn render_content_claims(out: &mut String, snapshot: &RepoSnapshot) {
    out.push_str("## Content Claims\n\n");
    if snapshot.claims.is_empty() {
        out.push_str("- No evidence-linked content claims were generated.\n\n");
        return;
    }

    let claim_index = ClaimRefIndex::new(&snapshot.claims);
    let observed = claim_index.strength_count(ClaimStrength::Observed);
    let inferred = claim_index.strength_count(ClaimStrength::Inferred);
    let suggested = claim_index.strength_count(ClaimStrength::Suggested);
    let blocked = claim_index.strength_count(ClaimStrength::Blocked);
    let boundary_kinds = claim_index.boundary_kinds();
    out.push_str(&format!(
        "- Summary: total `{}` / observed `{observed}` / inferred `{inferred}` / suggested `{suggested}` / blocked `{blocked}`\n",
        snapshot.claims.len()
    ));
    out.push_str(&format!(
        "- Boundary kinds: {}\n\n",
        boundary_kinds_or_none(&boundary_kinds)
    ));

    for claim in &snapshot.claims {
        out.push_str(&format!(
            "- `{}` `{:?}` route `{:?}` state `{:?}` evidence `{}`\n",
            claim.id,
            claim.strength,
            claim.route,
            claim.state,
            claim.evidence_ids.join("`, `")
        ));
        out.push_str(&format!(
            "  Allows: {}\n",
            debug_values_or_none(&claim.allowed_meanings)
        ));
        out.push_str(&format!(
            "  Boundaries: {}\n",
            debug_values_or_none(&claim.boundaries)
        ));
    }
    out.push('\n');
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

fn boundary_kinds_or_none(boundaries: &[ClaimBoundaryKind]) -> String {
    if boundaries.is_empty() {
        "none".to_string()
    } else {
        boundaries
            .iter()
            .map(|boundary| format!("`{boundary:?}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
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

fn decimal_confidence(value_x100: u8) -> String {
    if value_x100 >= 100 {
        "1.00".to_string()
    } else {
        format!("0.{value_x100:02}")
    }
}

fn decimal_prior(value_x1000: u16) -> String {
    format!("0.{value_x1000:03}")
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn routes_or_none(values: &[RouteKind]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .map(|route| format!("{route:?}"))
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
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
