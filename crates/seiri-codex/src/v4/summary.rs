use seiri_core::{
    CodexNativeAuditSummary, CodexNativeRouteSummary, RepoSnapshot, RoutePolicyBoundary,
};

pub(super) fn native_audit_summary(snapshot: &RepoSnapshot) -> CodexNativeAuditSummary {
    CodexNativeAuditSummary {
        entries_scanned: snapshot.entry_count,
        document_events: snapshot
            .readme_document
            .as_ref()
            .map_or(0, |document| document.events().len()),
        document_diagnostics: snapshot
            .readme_document
            .as_ref()
            .map_or(0, |document| document.diagnostics().len()),
        evidence_facts: snapshot.evidence_kernel.facts().len(),
        route_assessments: snapshot.route_assessments.len(),
        claims: snapshot.claims.len(),
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
        co_occurrence_gaps: snapshot.missing_route_priority.co_occurrence_gaps.len(),
        top_missing_route: snapshot.missing_route_priority.summary.top_route,
        top_missing_route_priority_x100: snapshot.missing_route_priority.summary.top_priority_x100,
    }
}

pub(super) fn native_route_summary(snapshot: &RepoSnapshot) -> CodexNativeRouteSummary {
    CodexNativeRouteSummary {
        assessments: snapshot.route_assessments.len(),
        root_structured_routes: snapshot
            .route_assessments
            .iter()
            .filter(|assessment| assessment.presence().root_structured())
            .count(),
        readme_routed_routes: snapshot
            .route_assessments
            .iter()
            .filter(|assessment| assessment.readme().routing().is_present())
            .count(),
        routes_with_repository_local_target: snapshot
            .route_assessments
            .iter()
            .filter(|assessment| {
                assessment
                    .readme()
                    .target_reachability()
                    .repository_local_present()
                    > 0
            })
            .count(),
        maintainer_decision_routes: snapshot
            .route_assessments
            .iter()
            .filter(|assessment| {
                assessment.policy() == RoutePolicyBoundary::MaintainerDecisionRequired
            })
            .count(),
    }
}
