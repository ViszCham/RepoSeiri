use seiri_core::{
    stable_id, GateKind, ImportantFileKind, MissingRoutePriority, PatchOperationKind, PatchPlan,
    PatchPlanBlockedItem, PatchPlanMode, PatchPlanOperation, PatchPlanSafetyPolicy,
    PatchPlanSource, PatchPlanSummary, PatchPreflightCheck, PatchPreflightCheckKind,
    PatchPreflightStatus, PatchProposal, PatchProposalDecision, PatchProposalIssueKind,
    PatchSafetyLevel, PatchTextEdit, ProfilePriority, RepoSnapshot, RouteKind, RouteState,
    Severity, TextEditSpan, TextEncoding,
};
use std::collections::BTreeSet;

const PLANNER_VERSION: &str = "safe_patch_planner.v3";

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
    let candidates = plan_candidates(snapshot);
    let candidate_count = candidates.len();
    let mut operations = Vec::new();
    let mut blocked = Vec::new();

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
            GateKind::Safe => match safe_operation(snapshot, &candidate, operations.len() + 1) {
                SafeDecision::Operation(operation) => operations.push(operation),
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
        planner_version: PLANNER_VERSION.to_string(),
        mode: PatchPlanMode::DryRun,
        profile: snapshot.profile.as_ref().map(|profile| profile.profile),
        safety_policy: safety_policy(),
        summary: summarize(candidate_count, &operations, &blocked),
        operations,
        blocked,
        claim_boundary: "Patch plan is a dry-run planning artifact. RepoSeiri v3 does not write files, invoke patch application, push branches, create PRs, choose policy, or guarantee popularity, trust, security, or quality. Safe operations carry typed proposals and require current-byte preflight before optional in-memory application. The base digest is a deterministic stale-base guard, not a cryptographic integrity or security guarantee.".to_string(),
    }
}

enum SafeDecision {
    Operation(PatchPlanOperation),
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
) -> SafeDecision {
    match candidate.pattern_id.as_str() {
        "common.docs.route_present" => plan_docs_route(snapshot, candidate, operation_index),
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

    SafeDecision::Operation(PatchPlanOperation {
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
    })
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
