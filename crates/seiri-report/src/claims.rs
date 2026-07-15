use seiri_core::{
    route_meaning_rule, ClaimStrength, ContentClaim, EvidenceId, MeaningAtom, MissingRoutePriority,
    RepositoryAnalysis, RouteAssessment, RouteState,
};

pub(crate) fn build_content_claims(snapshot: &RepositoryAnalysis) -> Vec<ContentClaim> {
    let mut claims = Vec::new();

    for assessment in &snapshot.route_assessments {
        push_route_claim(&mut claims, assessment);
    }

    for priority in &snapshot.missing_route_priority.priorities {
        push_priority_claim(&mut claims, priority);
    }

    claims
}

fn push_route_claim(claims: &mut Vec<ContentClaim>, assessment: &RouteAssessment) {
    let summary = assessment.summary_projection();
    let evidence_ids = normalized_evidence_ids(&assessment.summary_evidence_ids());
    if evidence_ids.is_empty() {
        return;
    }

    let rule = route_meaning_rule(assessment.route(), summary.state);
    let index = claims.len() + 1;
    claims.push(ContentClaim::new(
        index,
        assessment.route(),
        summary.state,
        route_state_strength(summary.state),
        evidence_ids,
        rule.indicates.to_vec(),
    ));
}

fn push_priority_claim(claims: &mut Vec<ContentClaim>, priority: &MissingRoutePriority) {
    let evidence_ids = normalized_evidence_ids(&priority.evidence_ids);
    if evidence_ids.is_empty() {
        return;
    }

    let rule = route_meaning_rule(priority.route, priority.state);
    let mut allowed_meanings = rule.indicates.to_vec();
    if !allowed_meanings.contains(&MeaningAtom::CalibrationCandidate) {
        allowed_meanings.push(MeaningAtom::CalibrationCandidate);
    }
    let index = claims.len() + 1;
    claims.push(ContentClaim::new(
        index,
        priority.route,
        priority.state,
        ClaimStrength::Suggested,
        evidence_ids,
        allowed_meanings,
    ));
}

fn route_state_strength(state: RouteState) -> ClaimStrength {
    match state {
        RouteState::Routed | RouteState::Structured | RouteState::Verified => {
            ClaimStrength::Observed
        }
        RouteState::UnsafeToInvent => ClaimStrength::Blocked,
        RouteState::Absent
        | RouteState::Implicit
        | RouteState::Weak
        | RouteState::Inherited
        | RouteState::Overridden
        | RouteState::Conflicting
        | RouteState::Overloaded
        | RouteState::Stale => ClaimStrength::Inferred,
    }
}

fn normalized_evidence_ids(ids: &[EvidenceId]) -> Vec<EvidenceId> {
    let mut evidence_ids = ids.to_vec();
    evidence_ids.sort();
    evidence_ids.dedup();
    evidence_ids
}
