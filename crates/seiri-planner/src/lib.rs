use seiri_core::{
    stable_id, GateKind, ImportantFileKind, MissingRoutePriority, PatchAnalysisRun,
    PatchBaseDigest, PatchOperationKind, PatchPlan, PatchPlanBlockedItem, PatchPlanMode,
    PatchPlanOperation, PatchPlanSafetyPolicy, PatchPlanSource, PatchPlanSummary,
    PatchPreflightCheck, PatchPreflightCheckKind, PatchPreflightStatus, PatchProposal,
    PatchProposalBinding, PatchProposalDecision, PatchProposalIssueKind, PatchSafetyLevel,
    PatchTextEdit, ProfilePriority, RepoSnapshot, RouteKind, RouteState, Severity,
    TextDocumentBase, TextEditSpan, TextEncoding,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

const LEGACY_PLANNER_VERSION: &str = "safe_patch_planner.v3";
const PLANNER_VERSION: &str = "safe_patch_planner.v4";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanBindingMode {
    CompatibilityV3,
    BoundV4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanCandidate {
    finding_id: Option<String>,
    pattern_id: String,
    title: String,
    gate: GateKind,
    source: PatchPlanSource,
    safety: PatchSafetyLevel,
    severity: Severity,
    priority: ProfilePriority,
    route: Option<RouteKind>,
    route_state: Option<RouteState>,
    suggested_kind: Option<PatchOperationKind>,
    weight: u32,
    reason: String,
}

#[must_use]
pub fn plan_safe_patches(snapshot: &RepoSnapshot) -> PatchPlan {
    build_plan(snapshot, PlanBindingMode::BoundV4)
}

/// Preserves the Q19 Codex compatibility projection without exposing Q34 bindings.
#[must_use]
pub fn plan_compatibility_safe_patches(snapshot: &RepoSnapshot) -> PatchPlan {
    build_plan(snapshot, PlanBindingMode::CompatibilityV3)
}

fn build_plan(snapshot: &RepoSnapshot, binding_mode: PlanBindingMode) -> PatchPlan {
    let candidates = plan_candidates(snapshot);
    let candidate_count = candidates.len();
    let mut operations = Vec::new();
    let mut blocked = Vec::new();
    let analysis_run = (binding_mode == PlanBindingMode::BoundV4).then(|| analysis_run(snapshot));

    for candidate in candidates {
        if candidate.route_state == Some(RouteState::UnsafeToInvent) {
            blocked.push(blocked_item(
                blocked.len() + 1,
                &candidate,
                "Route state is unsafe_to_invent, so RepoSeiri must not generate or preview a patch for it."
                    .to_string(),
                vec![
                    check(
                        PatchPreflightCheckKind::DryRunOnly,
                        PatchPreflightStatus::Pass,
                        "Planner is running in dry-run mode.",
                    ),
                    check(
                        PatchPreflightCheckKind::RouteSafeToInvent,
                        PatchPreflightStatus::Blocked,
                        "Route state is unsafe_to_invent.",
                    ),
                ],
            ));
            continue;
        }

        match candidate.gate {
            GateKind::Safe => match safe_operation(
                snapshot,
                &candidate,
                operations.len() + 1,
                binding_mode,
                analysis_run.as_ref(),
            ) {
                SafeDecision::Operation(operation) => operations.push(*operation),
                SafeDecision::Blocked {
                    reason,
                    preflight,
                    proposal,
                } => {
                    blocked.push(blocked_item_with_proposal(
                        blocked.len() + 1,
                        &candidate,
                        reason,
                        preflight,
                        proposal,
                    ));
                }
            },
            GateKind::Guarded => blocked.push(blocked_item(
                blocked.len() + 1,
                &candidate,
                "Guarded recommendation requires maintainer confirmation before a patch preview is generated."
                    .to_string(),
                gate_block_preflight(&candidate),
            )),
            GateKind::Manual => blocked.push(blocked_item(
                blocked.len() + 1,
                &candidate,
                "Manual recommendation requires human policy, legal, security, ownership, or product judgment before a patch preview is generated."
                    .to_string(),
                gate_block_preflight(&candidate),
            )),
        }
    }

    PatchPlan {
        schema_version: seiri_core::SCHEMA_VERSION.to_string(),
        planner_version: planner_version(binding_mode).to_string(),
        mode: PatchPlanMode::DryRun,
        profile: snapshot.profile.as_ref().map(|profile| profile.profile),
        safety_policy: safety_policy(),
        analysis_run,
        summary: summarize(candidate_count, &operations, &blocked),
        operations,
        blocked,
        claim_boundary: claim_boundary(binding_mode).to_string(),
    }
}

fn planner_version(binding_mode: PlanBindingMode) -> &'static str {
    match binding_mode {
        PlanBindingMode::CompatibilityV3 => LEGACY_PLANNER_VERSION,
        PlanBindingMode::BoundV4 => PLANNER_VERSION,
    }
}

fn claim_boundary(binding_mode: PlanBindingMode) -> &'static str {
    match binding_mode {
        PlanBindingMode::CompatibilityV3 => "Patch plan is a dry-run compatibility artifact. RepoSeiri does not write files, invoke patch application, push branches, create PRs, choose policy, or guarantee popularity, trust, security, or quality. Safe operations require current-byte preflight before optional in-memory application.",
        PlanBindingMode::BoundV4 => "Patch plan is a dry-run planning artifact. RepoSeiri does not write files, invoke patch application, push branches, create PRs, choose policy, or guarantee popularity, trust, security, or quality. Each Safe operation is bound to an analysis run, scanner base digest, and bounded anchor context after the current local source is rechecked. The FNV digests are deterministic stale-analysis guards, not cryptographic integrity or security guarantees.",
    }
}

enum SafeDecision {
    Operation(Box<PatchPlanOperation>),
    Blocked {
        reason: String,
        preflight: Vec<PatchPreflightCheck>,
        proposal: Option<PatchProposal>,
    },
}

fn plan_candidates(snapshot: &RepoSnapshot) -> Vec<PlanCandidate> {
    let mut candidates = Vec::new();
    let mut seen_patterns = BTreeSet::new();

    if let Some(profile) = &snapshot.profile {
        for recommendation in &profile.recommendations {
            let candidate = PlanCandidate {
                finding_id: recommendation.finding_id.clone(),
                pattern_id: recommendation.pattern_id.clone(),
                title: recommendation.title.clone(),
                gate: recommendation.gate,
                source: PatchPlanSource::ProfileRecommendation,
                safety: safety_for_gate(recommendation.gate),
                severity: recommendation.severity,
                priority: recommendation.priority,
                route: route_for_pattern_id(&recommendation.pattern_id),
                route_state: None,
                suggested_kind: operation_kind_for_pattern(&recommendation.pattern_id),
                weight: recommendation.weight,
                reason: recommendation.reason.clone(),
            };
            if seen_patterns.insert(candidate.pattern_id.clone()) {
                candidates.push(candidate);
            }
        }
    } else {
        for finding in &snapshot.findings {
            let Some(recommendation) = finding.recommendation.as_ref() else {
                continue;
            };
            let pattern_id = format!("finding.{}", finding.id);
            let candidate = PlanCandidate {
                finding_id: Some(finding.id.clone()),
                pattern_id,
                title: finding.title.clone(),
                gate: recommendation.gate,
                source: PatchPlanSource::FindingRecommendation,
                safety: safety_for_gate(recommendation.gate),
                severity: finding.severity,
                priority: priority_for_severity(finding.severity),
                route: None,
                route_state: None,
                suggested_kind: None,
                weight: 1,
                reason: recommendation.message.clone(),
            };
            if seen_patterns.insert(candidate.pattern_id.clone()) {
                candidates.push(candidate);
            }
        }
    }

    for priority in &snapshot.missing_route_priority.priorities {
        let pattern_id = route_priority_pattern_id(priority);
        let candidate = PlanCandidate {
            finding_id: None,
            pattern_id,
            title: format!("Review {:?} route priority", priority.route),
            gate: priority.gate,
            source: PatchPlanSource::MissingRoutePriority,
            safety: safety_for_gate(priority.gate),
            severity: priority.severity,
            priority: priority.priority,
            route: Some(priority.route),
            route_state: Some(priority.state),
            suggested_kind: operation_kind_for_route(priority.route),
            weight: u32::from(priority.priority_score_x100),
            reason: format!(
                "{} Missing route priority score {} / 100.",
                priority.reason, priority.priority_score_x100
            ),
        };
        if seen_patterns.insert(candidate.pattern_id.clone()) {
            candidates.push(candidate);
        }
    }

    candidates
}

fn safe_operation(
    snapshot: &RepoSnapshot,
    candidate: &PlanCandidate,
    operation_index: usize,
    binding_mode: PlanBindingMode,
    analysis_run: Option<&PatchAnalysisRun>,
) -> SafeDecision {
    match candidate.pattern_id.as_str() {
        "common.docs.route_present" => plan_docs_route(
            snapshot,
            candidate,
            operation_index,
            binding_mode,
            analysis_run,
        ),
        _ => SafeDecision::Blocked {
            reason: format!(
                "No Safe Patch Planner v3 operation exists for `{}`.",
                candidate.pattern_id
            ),
            preflight: vec![
                check(
                    PatchPreflightCheckKind::DryRunOnly,
                    PatchPreflightStatus::Pass,
                    "Planner is running in dry-run mode.",
                ),
                check(
                    PatchPreflightCheckKind::SafeGate,
                    PatchPreflightStatus::Pass,
                    "Candidate is Safe-gated.",
                ),
                check(
                    PatchPreflightCheckKind::SupportedOperation,
                    PatchPreflightStatus::Blocked,
                    unsupported_operation_detail(candidate),
                ),
            ],
            proposal: None,
        },
    }
}

fn plan_docs_route(
    snapshot: &RepoSnapshot,
    candidate: &PlanCandidate,
    operation_index: usize,
    binding_mode: PlanBindingMode,
    analysis_run: Option<&PatchAnalysisRun>,
) -> SafeDecision {
    let mut preflight = vec![
        check(
            PatchPreflightCheckKind::DryRunOnly,
            PatchPreflightStatus::Pass,
            "Planner is running in dry-run mode.",
        ),
        check(
            PatchPreflightCheckKind::SafeGate,
            PatchPreflightStatus::Pass,
            "Candidate is Safe-gated.",
        ),
        check(
            PatchPreflightCheckKind::SupportedOperation,
            PatchPreflightStatus::Pass,
            "README docs route insertion is the supported Safe preview-only operation.",
        ),
    ];

    let Some(readme) = &snapshot.readme else {
        preflight.push(check(
            PatchPreflightCheckKind::ExistingReadme,
            PatchPreflightStatus::Fail,
            "A safe README route patch requires an existing README.",
        ));
        return SafeDecision::Blocked {
            reason: "A safe README route patch requires an existing README. Creating README content is manual."
                .to_string(),
            preflight,
            proposal: None,
        };
    };
    preflight.push(check(
        PatchPreflightCheckKind::ExistingReadme,
        PatchPreflightStatus::Pass,
        "Existing README was detected.",
    ));

    let Some(document) = snapshot
        .readme_document
        .as_ref()
        .filter(|document| document.path() == readme.path)
    else {
        preflight.push(check(
            PatchPreflightCheckKind::BaseDigestBound,
            PatchPreflightStatus::Fail,
            "README scanner metadata is unavailable or does not match the summarized path.",
        ));
        return SafeDecision::Blocked {
            reason: "A typed patch proposal requires scanner-owned README base metadata."
                .to_string(),
            preflight,
            proposal: None,
        };
    };

    if readme
        .route_candidates
        .iter()
        .any(|candidate| candidate.route == RouteKind::Docs)
    {
        preflight.push(check(
            PatchPreflightCheckKind::ReadmeRouteAbsent,
            PatchPreflightStatus::Blocked,
            "README already exposes a docs route.",
        ));
        return SafeDecision::Blocked {
            reason: "README already exposes a docs route; no safe routing patch is needed."
                .to_string(),
            preflight,
            proposal: None,
        };
    }
    preflight.push(check(
        PatchPreflightCheckKind::ReadmeRouteAbsent,
        PatchPreflightStatus::Pass,
        "README does not already expose a docs route.",
    ));

    let Some(target) = docs_target(snapshot) else {
        preflight.push(check(
            PatchPreflightCheckKind::ExistingTarget,
            PatchPreflightStatus::Fail,
            "No existing docs directory was detected.",
        ));
        return SafeDecision::Blocked {
            reason: "A safe docs route patch requires an existing docs directory. Creating documentation content is guarded."
                .to_string(),
            preflight,
            proposal: None,
        };
    };
    preflight.push(check(
        PatchPreflightCheckKind::ExistingTarget,
        PatchPreflightStatus::Pass,
        "Existing docs directory was detected.",
    ));
    preflight.push(check(
        PatchPreflightCheckKind::NoPolicyContent,
        PatchPreflightStatus::Pass,
        "Operation only adds a route to existing content and does not invent policy text.",
    ));

    let base = document.base().clone();
    let eol = base.line_ending().sequence().unwrap_or("\n");
    let leading = if base.ends_with_line_ending() {
        eol.to_string()
    } else {
        format!("{eol}{eol}")
    };
    let replacement =
        format!("{leading}## Documentation{eol}{eol}- [Documentation]({target}){eol}");
    let proposal = PatchProposal::new(
        stable_id("patch-proposal", operation_index),
        readme.path.clone(),
        base.clone(),
        vec![PatchTextEdit::literal(
            stable_id("text-edit", operation_index),
            TextEditSpan::insertion(base.byte_len()),
            replacement,
        )],
    );
    let proposal_preflight = proposal.preflight_structure();
    preflight.extend(proposal_preflight_checks(&proposal));
    if proposal_preflight.decision != PatchProposalDecision::Ready {
        return SafeDecision::Blocked {
            reason: format!(
                "Typed patch proposal is {:?}; review its encoding, EOL, span, and policy-slot preflight before application.",
                proposal_preflight.decision
            ),
            preflight,
            proposal: Some(proposal),
        };
    }

    let binding = match binding_mode {
        PlanBindingMode::CompatibilityV3 => None,
        PlanBindingMode::BoundV4 => {
            let Some(analysis_run) = analysis_run else {
                return SafeDecision::Blocked {
                    reason: "Bound planner did not retain an analysis run; no patch preview was generated."
                        .to_string(),
                    preflight: blocked_analysis_preflight(
                        preflight,
                        PatchPreflightCheckKind::AnalysisRunBound,
                        "Bound planner requires a retained analysis run.",
                    ),
                    proposal: None,
                };
            };
            match bind_current_proposal(snapshot, &proposal, analysis_run.clone()) {
                Ok(binding) => {
                    preflight.push(check(
                        PatchPreflightCheckKind::CurrentAnalysisInput,
                        PatchPreflightStatus::Pass,
                        "Current repository-local README bytes match the scanner-owned base before planning.",
                    ));
                    preflight.push(check(
                        PatchPreflightCheckKind::AnalysisRunBound,
                        PatchPreflightStatus::Pass,
                        format!(
                            "Proposal is bound to analysis run `{}` with snapshot digest {}.",
                            binding.analysis_run.id, binding.analysis_run.snapshot_digest
                        ),
                    ));
                    preflight.push(check(
                        PatchPreflightCheckKind::AnchorContextBound,
                        PatchPreflightStatus::Pass,
                        format!(
                            "Proposal retains {} bounded anchor context(s) without retaining source text.",
                            binding.anchors.len()
                        ),
                    ));
                    Some(binding)
                }
                Err(reason) => {
                    return SafeDecision::Blocked {
                        reason: format!(
                            "Current README bytes could not be bound to this analysis run; no patch preview was generated. {reason}"
                        ),
                        preflight: blocked_analysis_preflight(
                            preflight,
                            PatchPreflightCheckKind::CurrentAnalysisInput,
                            &reason,
                        ),
                        proposal: None,
                    };
                }
            }
        }
    };

    SafeDecision::Operation(Box::new(PatchPlanOperation {
        id: stable_id("patch-op", operation_index),
        gate: GateKind::Safe,
        kind: PatchOperationKind::AddReadmeRoute,
        source: candidate.source,
        safety: PatchSafetyLevel::PreviewOnly,
        priority: candidate.priority,
        title: "Add README documentation route".to_string(),
        path: readme.path.clone(),
        route: Some(RouteKind::Docs),
        finding_id: candidate.finding_id.clone(),
        pattern_id: candidate.pattern_id.clone(),
        preview_only: true,
        requires_confirmation: true,
        rationale: format!(
            "{} This preview only adds routing to an existing documentation target.",
            candidate.reason
        ),
        planned_change: format!("Append a Documentation section linking to `{target}`."),
        proposal,
        binding,
        preflight,
        diff_preview: vec![
            format!("--- {}", readme.path),
            format!("+++ {}", readme.path),
            "@@ end_of_file @@".to_string(),
            "+".to_string(),
            "+## Documentation".to_string(),
            "+".to_string(),
            format!("+- [Documentation]({target})"),
        ],
    }))
}

fn blocked_analysis_preflight(
    mut preflight: Vec<PatchPreflightCheck>,
    kind: PatchPreflightCheckKind,
    detail: &str,
) -> Vec<PatchPreflightCheck> {
    preflight.push(check(kind, PatchPreflightStatus::Fail, detail));
    preflight
}

fn bind_current_proposal(
    snapshot: &RepoSnapshot,
    proposal: &PatchProposal,
    analysis_run: PatchAnalysisRun,
) -> Result<PatchProposalBinding, String> {
    let current = read_current_document_bytes(snapshot, &proposal.path)?;
    let current_base = TextDocumentBase::from_bytes(&current);
    if current_base != proposal.base {
        return Err(format!(
            "Scanner base {} ({} bytes) differs from current source base {} ({} bytes).",
            proposal.base.digest(),
            proposal.base.byte_len(),
            current_base.digest(),
            current_base.byte_len(),
        ));
    }
    PatchProposalBinding::bind(analysis_run, proposal, &current)
        .map_err(|error| format!("Binding construction failed: {error}"))
}

fn read_current_document_bytes(
    snapshot: &RepoSnapshot,
    relative_path: &str,
) -> Result<Vec<u8>, String> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err("Planner refused a non-repository-relative document path.".to_string());
    }

    let root = seiri_fs::RepositoryRoot::resolve(Path::new(&snapshot.repo_root))
        .map_err(|error| format!("Repository root could not be resolved: {error}"))?;
    let candidate = root.as_path().join(relative);
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("Current document could not be resolved: {error}"))?;
    if !canonical.starts_with(root.as_path()) {
        return Err(
            "Planner refused a document whose resolved path escapes the repository root."
                .to_string(),
        );
    }
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Current document metadata could not be read: {error}"))?;
    if !metadata.is_file() {
        return Err(
            "Planner requires the current document target to be a regular file.".to_string(),
        );
    }
    fs::read(&canonical).map_err(|error| format!("Current document could not be read: {error}"))
}

fn analysis_run(snapshot: &RepoSnapshot) -> PatchAnalysisRun {
    let mut material = Vec::with_capacity(snapshot.repo_root.len() + 96);
    append_run_field(&mut material, &snapshot.schema_version);
    append_run_field(&mut material, &snapshot.repo_root);
    append_run_field(&mut material, &snapshot.entry_count.to_string());
    append_run_field(&mut material, &snapshot.files.len().to_string());
    append_run_field(&mut material, &snapshot.important_files.len().to_string());
    append_run_field(&mut material, &snapshot.route_assessments.len().to_string());
    append_run_field(&mut material, &snapshot.claims.len().to_string());
    append_run_field(&mut material, &snapshot.findings.len().to_string());
    if let Some(readme) = &snapshot.readme_document {
        append_run_field(&mut material, readme.path());
        append_run_field(&mut material, &readme.base().digest().to_string());
    }
    let snapshot_digest = PatchBaseDigest::from_bytes(&material);
    PatchAnalysisRun::new(
        format!("analysis-run-{:016x}", snapshot_digest.as_u64()),
        snapshot_digest,
    )
}

fn append_run_field(material: &mut Vec<u8>, value: &str) {
    let byte_len =
        u64::try_from(value.len()).expect("usize always fits into u64 on supported targets");
    material.extend_from_slice(&byte_len.to_le_bytes());
    material.extend_from_slice(value.as_bytes());
}

fn docs_target(snapshot: &RepoSnapshot) -> Option<String> {
    snapshot
        .important_files
        .iter()
        .find(|file| file.kind == ImportantFileKind::DocsDirectory)
        .map(|file| format!("{}/", file.path.trim_end_matches('/')))
}

fn proposal_preflight_checks(proposal: &PatchProposal) -> Vec<PatchPreflightCheck> {
    let result = proposal.preflight_structure();
    let encoding_status = if proposal.base.encoding() == TextEncoding::Unknown {
        PatchPreflightStatus::Fail
    } else {
        PatchPreflightStatus::Pass
    };
    let eol_held = result.has_issue(PatchProposalIssueKind::MixedLineEndings)
        || result.has_issue(PatchProposalIssueKind::MissingLineEndingConvention);
    let span_failed = result.has_issue(PatchProposalIssueKind::SpanOutOfBounds)
        || result.has_issue(PatchProposalIssueKind::OverlappingSpans)
        || result.has_issue(PatchProposalIssueKind::OutputLengthOverflow);
    let policy_held = result.has_issue(PatchProposalIssueKind::UnresolvedPolicySlot);
    let ready_status = match result.decision {
        PatchProposalDecision::Ready => PatchPreflightStatus::Pass,
        PatchProposalDecision::Hold => PatchPreflightStatus::Blocked,
        PatchProposalDecision::Reject => PatchPreflightStatus::Fail,
    };

    vec![
        check(
            PatchPreflightCheckKind::BaseDigestBound,
            PatchPreflightStatus::Pass,
            format!(
                "Proposal is bound to base digest {} and {} bytes; current bytes must be rechecked before application.",
                proposal.base.digest(),
                proposal.base.byte_len()
            ),
        ),
        check(
            PatchPreflightCheckKind::EncodingKnown,
            encoding_status,
            format!("Base encoding is {:?}.", proposal.base.encoding()),
        ),
        check(
            PatchPreflightCheckKind::LineEndingBound,
            if eol_held {
                PatchPreflightStatus::Blocked
            } else {
                PatchPreflightStatus::Pass
            },
            format!("Base line ending is {:?}.", proposal.base.line_ending()),
        ),
        check(
            PatchPreflightCheckKind::NonOverlappingSpans,
            if span_failed {
                PatchPreflightStatus::Fail
            } else {
                PatchPreflightStatus::Pass
            },
            "Text edit spans are checked for bounds, overlap, and output length overflow.",
        ),
        check(
            PatchPreflightCheckKind::PolicySlotsResolved,
            if policy_held {
                PatchPreflightStatus::Blocked
            } else {
                PatchPreflightStatus::Pass
            },
            "Unresolved policy slots hold a proposal before application.",
        ),
        check(
            PatchPreflightCheckKind::ProposalReady,
            ready_status,
            format!(
                "Structural patch proposal decision is {:?}; stale-base and UTF-8 boundary checks run against current bytes before application.",
                result.decision
            ),
        ),
    ]
}

fn blocked_item(
    index: usize,
    candidate: &PlanCandidate,
    reason: String,
    preflight: Vec<PatchPreflightCheck>,
) -> PatchPlanBlockedItem {
    blocked_item_with_proposal(index, candidate, reason, preflight, None)
}

fn blocked_item_with_proposal(
    index: usize,
    candidate: &PlanCandidate,
    reason: String,
    preflight: Vec<PatchPreflightCheck>,
    proposal: Option<PatchProposal>,
) -> PatchPlanBlockedItem {
    PatchPlanBlockedItem {
        id: stable_id("patch-blocked", index),
        gate: candidate.gate,
        source: candidate.source,
        safety: candidate.safety,
        severity: candidate.severity,
        priority: candidate.priority,
        title: candidate.title.clone(),
        route: candidate.route,
        finding_id: candidate.finding_id.clone(),
        pattern_id: candidate.pattern_id.clone(),
        suggested_kind: candidate.suggested_kind,
        proposal,
        reason,
        preflight,
    }
}

fn summarize(
    total_candidates: usize,
    operations: &[PatchPlanOperation],
    blocked: &[PatchPlanBlockedItem],
) -> PatchPlanSummary {
    let operation_checks = operations
        .iter()
        .flat_map(|operation| operation.preflight.iter());
    let blocked_checks = blocked.iter().flat_map(|item| item.preflight.iter());
    let mut preflight_passed = 0;
    let mut preflight_failed = 0;
    for check in operation_checks.chain(blocked_checks) {
        if check.status == PatchPreflightStatus::Pass {
            preflight_passed += 1;
        } else {
            preflight_failed += 1;
        }
    }

    PatchPlanSummary {
        total_candidates,
        safe_operations: operations.len(),
        safe_blocked: blocked
            .iter()
            .filter(|item| item.gate == GateKind::Safe)
            .count(),
        guarded_items: blocked
            .iter()
            .filter(|item| item.gate == GateKind::Guarded)
            .count(),
        manual_items: blocked
            .iter()
            .filter(|item| item.gate == GateKind::Manual)
            .count(),
        preview_only_operations: operations
            .iter()
            .filter(|operation| operation.preview_only)
            .count(),
        preflight_passed,
        preflight_failed,
    }
}

fn gate_block_preflight(candidate: &PlanCandidate) -> Vec<PatchPreflightCheck> {
    vec![
        check(
            PatchPreflightCheckKind::DryRunOnly,
            PatchPreflightStatus::Pass,
            "Planner is running in dry-run mode.",
        ),
        check(
            PatchPreflightCheckKind::SafeGate,
            PatchPreflightStatus::Blocked,
            gate_block_detail(candidate),
        ),
    ]
}

fn safety_policy() -> PatchPlanSafetyPolicy {
    PatchPlanSafetyPolicy {
        version: PLANNER_VERSION.to_string(),
        writes_files: false,
        applies_patches: false,
        safe_gate_only: true,
        requires_existing_targets: true,
        blocks_unsafe_to_invent: true,
        guarded_and_manual_are_blocked: true,
    }
}

fn safety_for_gate(gate: GateKind) -> PatchSafetyLevel {
    match gate {
        GateKind::Safe => PatchSafetyLevel::PreviewOnly,
        GateKind::Guarded => PatchSafetyLevel::ReviewRequired,
        GateKind::Manual => PatchSafetyLevel::ManualOnly,
    }
}

fn route_priority_pattern_id(priority: &MissingRoutePriority) -> String {
    priority
        .baseline_pattern_ids
        .first()
        .or_else(|| priority.candidate_pattern_ids.first())
        .cloned()
        .unwrap_or_else(|| format!("route_priority.{}", route_slug(priority.route)))
}

fn operation_kind_for_pattern(pattern_id: &str) -> Option<PatchOperationKind> {
    match pattern_id {
        "common.docs.route_present" => Some(PatchOperationKind::AddReadmeRoute),
        "common.support.route_present" => Some(PatchOperationKind::AddSupportSkeletonDraft),
        "common.security.route_present" => Some(PatchOperationKind::AddSecuritySkeletonDraft),
        "common.release.route_present" => Some(PatchOperationKind::MoveReadmeDetailToDocsDraft),
        "common.lifecycle.route_present" | "LIF-001" => Some(PatchOperationKind::AddLifecycleRoute),
        "common.license.file_present" => Some(PatchOperationKind::AddClaimBoundaryNote),
        _ => route_for_pattern_id(pattern_id).and_then(operation_kind_for_route),
    }
}

fn operation_kind_for_route(route: RouteKind) -> Option<PatchOperationKind> {
    match route {
        RouteKind::Docs | RouteKind::Quickstart | RouteKind::Contributing => {
            Some(PatchOperationKind::AddReadmeRoute)
        }
        RouteKind::Support | RouteKind::Intake => Some(PatchOperationKind::AddSupportSkeletonDraft),
        RouteKind::Security => Some(PatchOperationKind::AddSecuritySkeletonDraft),
        RouteKind::Release => Some(PatchOperationKind::MoveReadmeDetailToDocsDraft),
        RouteKind::Lifecycle => Some(PatchOperationKind::AddLifecycleRoute),
        RouteKind::License | RouteKind::Governance | RouteKind::Ownership => {
            Some(PatchOperationKind::AddClaimBoundaryNote)
        }
        RouteKind::Identity | RouteKind::Automation | RouteKind::Hygiene | RouteKind::Unknown => {
            None
        }
    }
}

fn unsupported_operation_detail(candidate: &PlanCandidate) -> String {
    candidate.suggested_kind.map_or_else(
        || "This pattern has no deterministic preview-only operation.".to_string(),
        |kind| {
            format!(
                "`{kind:?}` exists as a typed Q8 operation candidate, but it is not eligible for Safe preview generation from this evidence."
            )
        },
    )
}

fn gate_block_detail(candidate: &PlanCandidate) -> String {
    match (candidate.gate, candidate.suggested_kind) {
        (GateKind::Guarded, Some(kind)) => format!(
            "`{kind:?}` is a review-required Q8 operation candidate. RepoSeiri records it but does not generate or apply it without maintainer confirmation."
        ),
        (GateKind::Manual, Some(kind)) => format!(
            "`{kind:?}` is blocked behind human policy, legal, security, ownership, contact, or publication judgment."
        ),
        _ => format!(
            "Candidate is {:?}-gated and is not eligible for automatic patch preview.",
            candidate.gate
        ),
    }
}

fn route_for_pattern_id(pattern_id: &str) -> Option<RouteKind> {
    match pattern_id {
        "common.identity.readme_present" => Some(RouteKind::Identity),
        "common.docs.route_present" => Some(RouteKind::Docs),
        "common.quickstart.route_present" => Some(RouteKind::Quickstart),
        "common.support.route_present" => Some(RouteKind::Support),
        "common.contributing.route_present" => Some(RouteKind::Contributing),
        "common.security.route_present" => Some(RouteKind::Security),
        "common.release.route_present" => Some(RouteKind::Release),
        "common.lifecycle.route_present" => Some(RouteKind::Lifecycle),
        "common.automation.route_present" => Some(RouteKind::Automation),
        "common.license.file_present" => Some(RouteKind::License),
        "LIF-001" => Some(RouteKind::Lifecycle),
        _ => None,
    }
}

fn route_slug(route: RouteKind) -> &'static str {
    match route {
        RouteKind::Identity => "identity",
        RouteKind::Docs => "docs",
        RouteKind::Quickstart => "quickstart",
        RouteKind::Support => "support",
        RouteKind::Intake => "intake",
        RouteKind::Contributing => "contributing",
        RouteKind::Security => "security",
        RouteKind::Release => "release",
        RouteKind::Lifecycle => "lifecycle",
        RouteKind::Governance => "governance",
        RouteKind::License => "license",
        RouteKind::Automation => "automation",
        RouteKind::Ownership => "ownership",
        RouteKind::Hygiene => "hygiene",
        RouteKind::Unknown => "unknown",
    }
}

fn check(
    kind: PatchPreflightCheckKind,
    status: PatchPreflightStatus,
    detail: impl Into<String>,
) -> PatchPreflightCheck {
    PatchPreflightCheck {
        kind,
        status,
        detail: detail.into(),
    }
}

fn priority_for_severity(severity: Severity) -> ProfilePriority {
    match severity {
        Severity::Info => ProfilePriority::Low,
        Severity::Low => ProfilePriority::Normal,
        Severity::Medium => ProfilePriority::High,
        Severity::High => ProfilePriority::Critical,
    }
}
