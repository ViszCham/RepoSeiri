use seiri_core::{
    stable_id, BaselineStatus, CalibrationRun, CodexReviewContext, Evidence, EvidenceKind,
    EvidenceSource, ImportantFileKind, PatchPlan, ProfileKind, RepoSnapshot, RouteKind,
};
use seiri_fs::RepoFsScan;
use std::fmt::{Display, Formatter};
use std::path::Path;

#[derive(Debug)]
pub enum AuditError {
    Fs(seiri_fs::FsError),
    Markdown(seiri_markdown::MarkdownError),
    Calibration(seiri_calibration::CalibrationError),
    Json(serde_json::Error),
}

impl Display for AuditError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fs(error) => write!(f, "{error}"),
            Self::Markdown(error) => write!(f, "{error}"),
            Self::Calibration(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
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
    let baseline = seiri_patterns::evaluate_common_baseline(&snapshot);
    snapshot.pattern_matches = baseline.pattern_matches;
    snapshot.findings = baseline.findings;
    snapshot.baseline = Some(baseline.report);
    snapshot.profile = seiri_profiles::evaluate_profile(&snapshot, profile);
    Ok(snapshot)
}

pub fn to_json(snapshot: &RepoSnapshot) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(snapshot)?)
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

pub fn calibrate_dataset_path(path: impl AsRef<Path>) -> Result<CalibrationRun, AuditError> {
    let dataset = seiri_calibration::load_dataset(path)?;
    Ok(seiri_calibration::calibrate_dataset(&dataset))
}

pub fn calibration_to_json(run: &CalibrationRun) -> Result<String, AuditError> {
    Ok(serde_json::to_string_pretty(run)?)
}

pub fn codex_repository_with_profile(
    path: impl AsRef<Path>,
    profile: ProfileKind,
) -> Result<CodexReviewContext, AuditError> {
    let snapshot = audit_repository_with_profile(path, profile)?;
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    Ok(seiri_codex::build_review_context(&snapshot, &plan))
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
    let mut out = String::new();
    out.push_str("# RepoSeiri Calibration Report\n\n");
    out.push_str(&format!("- Schema: `{}`\n", run.schema_version));
    out.push_str(&format!("- Dataset: `{}`\n", run.dataset_id));
    out.push_str(&format!("- Records: `{}`\n", run.summary.records));
    out.push_str(&format!(
        "- Known pattern stats: `{}`\n",
        run.summary.known_pattern_stats
    ));
    out.push_str(&format!(
        "- Pending patterns: `{}`\n",
        run.summary.pending_patterns
    ));
    out.push_str(&format!(
        "- Weight suggestions: `{}`\n",
        run.summary.weight_suggestions
    ));
    out.push_str(&format!("- Boundary: {}\n\n", run.claim_boundary));

    out.push_str("## Pattern Stats\n\n");
    if run.stats.is_empty() {
        out.push_str("- No known pattern stats generated.\n\n");
    } else {
        for stat in &run.stats {
            out.push_str(&format!("### `{}`\n\n", stat.pattern_id));
            out.push_str(&format!("- Repositories: `{}`\n", stat.repositories));
            out.push_str(&format!("- Observations: `{}`\n", stat.observations));
            out.push_str(&format!("- Frequency x1000: `{}`\n", stat.frequency_x1000));
            out.push_str(&format!("- Confidence: `{:?}`\n", stat.confidence));
            out.push_str(&format!("- Note: {}\n\n", stat.confidence_note));
        }
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
            out.push_str(&format!(
                "- `{}` `{}` profile `{}` current `{}` suggested `{}` delta `{}` confidence `{:?}` status `{:?}`\n",
                suggestion.id,
                suggestion.pattern_id,
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
    out.push_str(&format!("- Mode: `{:?}`\n", plan.mode));
    match plan.profile {
        Some(profile) => out.push_str(&format!("- Profile: `{profile}`\n")),
        None => out.push_str("- Profile: not selected\n"),
    }
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
    out.push_str(&format!("- Boundary: {}\n\n", plan.claim_boundary));

    out.push_str("## Safe Operations\n\n");
    if plan.operations.is_empty() {
        out.push_str("- No safe operations generated.\n\n");
    } else {
        for operation in &plan.operations {
            out.push_str(&format!("### {}\n\n", operation.id));
            out.push_str(&format!("- Gate: `{:?}`\n", operation.gate));
            out.push_str(&format!("- Kind: `{:?}`\n", operation.kind));
            out.push_str(&format!("- Path: `{}`\n", operation.path));
            out.push_str(&format!("- Pattern: `{}`\n", operation.pattern_id));
            if let Some(finding_id) = &operation.finding_id {
                out.push_str(&format!("- Finding: `{finding_id}`\n"));
            }
            out.push_str(&format!("- Change: {}\n", operation.planned_change));
            out.push_str(&format!("- Rationale: {}\n\n", operation.rationale));
            out.push_str("```diff\n");
            for line in &operation.diff_preview {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("```\n\n");
        }
    }

    out.push_str("## Blocked Items\n\n");
    if plan.blocked.is_empty() {
        out.push_str("- No blocked items.\n");
    } else {
        for item in &plan.blocked {
            out.push_str(&format!(
                "- `{}` `{:?}` `{}`: {}\n",
                item.id, item.gate, item.pattern_id, item.reason
            ));
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
    }
    out.push_str(&format!(
        "- Evidence items: `{}`\n",
        snapshot.evidence.len()
    ));
    out.push_str(&format!("- Findings: `{}`\n\n", snapshot.findings.len()));

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
        }
        None => out.push_str("- Path: not found\n\n"),
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
        ImportantFileKind::Changelog => Some(RouteKind::Release),
        ImportantFileKind::Codeowners => Some(RouteKind::Ownership),
        ImportantFileKind::CargoToml => Some(RouteKind::Identity),
        ImportantFileKind::DocsDirectory => Some(RouteKind::Docs),
        ImportantFileKind::Workflow => Some(RouteKind::Automation),
    }
}
