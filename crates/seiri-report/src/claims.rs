use seiri_core::{
    route_meaning_rule, ClaimStrength, ContentClaim, EvidenceId, MeaningAtom, MissingRoutePriority,
    RepoSnapshot, RouteState, RouteStateReport,
};

pub(crate) fn build_content_claims(snapshot: &RepoSnapshot) -> Vec<ContentClaim> {
    let mut claims = Vec::new();

    for route_state in &snapshot.route_states {
        push_route_state_claim(&mut claims, route_state);
    }

    for priority in &snapshot.missing_route_priority.priorities {
        push_priority_claim(&mut claims, priority);
    }

    claims
}

fn push_route_state_claim(claims: &mut Vec<ContentClaim>, route_state: &RouteStateReport) {
    let evidence_ids = normalized_evidence_ids(&route_state.evidence_ids);
    if evidence_ids.is_empty() {
        return;
    }

    let rule = route_meaning_rule(route_state.route, route_state.state);
    let index = claims.len() + 1;
    claims.push(ContentClaim::new(
        index,
        route_state.route,
        route_state.state,
        route_state_strength(route_state.state),
        evidence_ids,
        rule.indicates.to_vec(),
        rule.does_not_indicate.to_vec(),
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
        rule.does_not_indicate.to_vec(),
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
    let mut evidence_ids = ids
        .iter()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    evidence_ids.sort();
    evidence_ids.dedup();
    evidence_ids
}
