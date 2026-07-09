use seiri_core::{
    stable_id, GateKind, ImportantFileKind, PatchOperationKind, PatchPlan, PatchPlanBlockedItem,
    PatchPlanMode, PatchPlanOperation, PatchPlanSummary, ProfilePriority, RepoSnapshot, RouteKind,
    Severity,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanCandidate {
    finding_id: Option<String>,
    pattern_id: String,
    title: String,
    gate: GateKind,
    severity: Severity,
    priority: ProfilePriority,
    weight: u32,
    reason: String,
}

#[must_use]
pub fn plan_safe_patches(snapshot: &RepoSnapshot) -> PatchPlan {
    let candidates = plan_candidates(snapshot);
    let mut operations = Vec::new();
    let mut blocked = Vec::new();

    for candidate in candidates {
        match candidate.gate {
            GateKind::Safe => match safe_operation(snapshot, &candidate, operations.len() + 1) {
                SafeDecision::Operation(operation) => operations.push(operation),
                SafeDecision::Blocked(reason) => {
                    blocked.push(blocked_item(blocked.len() + 1, &candidate, reason));
                }
            },
            GateKind::Guarded => blocked.push(blocked_item(
                blocked.len() + 1,
                &candidate,
                "Guarded recommendation requires maintainer confirmation before a patch is generated."
                    .to_string(),
            )),
            GateKind::Manual => blocked.push(blocked_item(
                blocked.len() + 1,
                &candidate,
                "Manual recommendation requires human policy or product judgment before a patch is generated."
                    .to_string(),
            )),
        }
    }

    PatchPlan {
        schema_version: seiri_core::SCHEMA_VERSION.to_string(),
        mode: PatchPlanMode::DryRun,
        profile: snapshot.profile.as_ref().map(|profile| profile.profile),
        summary: summarize(&operations, &blocked),
        operations,
        blocked,
        claim_boundary: "Patch plan is a dry-run planning artifact. RepoSeiri does not write files, push branches, create PRs, choose policy, or guarantee popularity, trust, security, or quality in Block D.".to_string(),
    }
}

enum SafeDecision {
    Operation(PatchPlanOperation),
    Blocked(String),
}

fn plan_candidates(snapshot: &RepoSnapshot) -> Vec<PlanCandidate> {
    if let Some(profile) = &snapshot.profile {
        return profile
            .recommendations
            .iter()
            .map(|recommendation| PlanCandidate {
                finding_id: recommendation.finding_id.clone(),
                pattern_id: recommendation.pattern_id.clone(),
                title: recommendation.title.clone(),
                gate: recommendation.gate,
                severity: recommendation.severity,
                priority: recommendation.priority,
                weight: recommendation.weight,
                reason: recommendation.reason.clone(),
            })
            .collect();
    }

    snapshot
        .findings
        .iter()
        .filter_map(|finding| {
            let recommendation = finding.recommendation.as_ref()?;
            Some(PlanCandidate {
                finding_id: Some(finding.id.clone()),
                pattern_id: format!("finding.{}", finding.id),
                title: finding.title.clone(),
                gate: recommendation.gate,
                severity: finding.severity,
                priority: priority_for_severity(finding.severity),
                weight: 1,
                reason: recommendation.message.clone(),
            })
        })
        .collect()
}

fn safe_operation(
    snapshot: &RepoSnapshot,
    candidate: &PlanCandidate,
    operation_index: usize,
) -> SafeDecision {
    match candidate.pattern_id.as_str() {
        "common.docs.route_present" => plan_docs_route(snapshot, candidate, operation_index),
        _ => SafeDecision::Blocked(format!(
            "No safe dry-run operation exists for `{}` in Block D.",
            candidate.pattern_id
        )),
    }
}

fn plan_docs_route(
    snapshot: &RepoSnapshot,
    candidate: &PlanCandidate,
    operation_index: usize,
) -> SafeDecision {
    let Some(readme) = &snapshot.readme else {
        return SafeDecision::Blocked(
            "A safe README route patch requires an existing README. Creating README content is manual."
                .to_string(),
        );
    };
    if readme
        .route_candidates
        .iter()
        .any(|candidate| candidate.route == RouteKind::Docs)
    {
        return SafeDecision::Blocked(
            "README already exposes a docs route; no safe routing patch is needed.".to_string(),
        );
    }

    let Some(target) = docs_target(snapshot) else {
        return SafeDecision::Blocked(
            "A safe docs route patch requires an existing docs directory. Creating documentation content is guarded."
                .to_string(),
        );
    };

    SafeDecision::Operation(PatchPlanOperation {
        id: stable_id("patch-op", operation_index),
        gate: GateKind::Safe,
        kind: PatchOperationKind::AddReadmeRoute,
        title: "Add README documentation route".to_string(),
        path: readme.path.clone(),
        route: Some(RouteKind::Docs),
        finding_id: candidate.finding_id.clone(),
        pattern_id: candidate.pattern_id.clone(),
        rationale: format!(
            "{} This operation only adds routing to an existing documentation target.",
            candidate.reason
        ),
        planned_change: format!("Append a Documentation section linking to `{target}`."),
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

fn blocked_item(index: usize, candidate: &PlanCandidate, reason: String) -> PatchPlanBlockedItem {
    PatchPlanBlockedItem {
        id: stable_id("patch-blocked", index),
        gate: candidate.gate,
        title: candidate.title.clone(),
        finding_id: candidate.finding_id.clone(),
        pattern_id: candidate.pattern_id.clone(),
        reason,
    }
}

fn summarize(
    operations: &[PatchPlanOperation],
    blocked: &[PatchPlanBlockedItem],
) -> PatchPlanSummary {
    PatchPlanSummary {
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
