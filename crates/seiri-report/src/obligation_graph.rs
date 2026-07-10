use seiri_core::{
    ConditionalObligation, CoverageIncompleteReason, CoverageScope, CoverageStatus,
    DocumentConflict, DocumentConflictSide, DocumentConsistencyError, DocumentConsistencyReport,
    DocumentId, EvidenceFact, EvidenceId, EvidenceKind, EvidenceSet, Observation, RepoSnapshot,
    RepositoryFacet, RouteKind,
};
use std::collections::BTreeMap;

const MAX_DOCUMENT_CONFLICTS: usize = 64;
const MAX_ROUTE_TARGET_GROUPS: usize = 128;

pub(crate) fn build_document_consistency_report(
    snapshot: &RepoSnapshot,
) -> Result<DocumentConsistencyReport, DocumentConsistencyError> {
    let mut obligations = build_conditional_obligations(snapshot);
    obligations.sort_by(|left, right| left.id.cmp(&right.id));

    let conflict_build = build_document_conflicts(snapshot)?;
    let mut conflicts = conflict_build.conflicts;
    conflicts.sort_by(|left, right| left.id.cmp(&right.id));

    let conflict_coverage = if conflict_build.truncated {
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    } else {
        snapshot
            .coverage
            .record(CoverageScope::MarkdownDocuments)
            .map_or(CoverageStatus::NotRequested, |record| record.status)
    };
    DocumentConsistencyReport::try_new(obligations, conflicts, conflict_coverage)
}

fn build_conditional_obligations(snapshot: &RepoSnapshot) -> Vec<ConditionalObligation> {
    let mut obligations = Vec::new();
    for facet in RepositoryFacet::ALL {
        let Some(reason_ids) = snapshot.facets.observed_evidence(facet) else {
            continue;
        };
        let reason = EvidenceSet::try_new(reason_ids.to_vec())
            .expect("observed facet assessments retain non-empty evidence");
        for route in routes_for_facet(facet) {
            let observation = route_observation(snapshot, *route);
            obligations.push(ConditionalObligation::new(
                facet,
                *route,
                reason.clone(),
                observation,
            ));
        }
    }
    obligations
}

fn routes_for_facet(facet: RepositoryFacet) -> &'static [RouteKind] {
    match facet {
        RepositoryFacet::Package => &[RouteKind::Docs, RouteKind::Quickstart],
        RepositoryFacet::Binary => &[RouteKind::Quickstart, RouteKind::Release],
        RepositoryFacet::Infrastructure => &[RouteKind::Security, RouteKind::Automation],
        RepositoryFacet::Documentation => &[RouteKind::Docs, RouteKind::Support],
        RepositoryFacet::Research => &[RouteKind::Docs, RouteKind::Quickstart],
        RepositoryFacet::Template => &[RouteKind::Quickstart, RouteKind::Contributing],
        RepositoryFacet::Product => &[RouteKind::Docs, RouteKind::Support],
    }
}

fn route_observation(snapshot: &RepoSnapshot, route: RouteKind) -> Observation<()> {
    let evidence = snapshot
        .route_states
        .iter()
        .find(|state| state.route == route)
        .map_or_else(Vec::new, |state| state.evidence_ids.clone());
    if evidence.is_empty() {
        snapshot
            .coverage
            .observe_absence(CoverageScope::RepositoryFiles)
    } else {
        Observation::present((), evidence)
            .expect("route state evidence ids are non-empty after collection")
    }
}

fn build_document_conflicts(
    snapshot: &RepoSnapshot,
) -> Result<ConflictBuild, DocumentConsistencyError> {
    let mut by_route_target = BTreeMap::<(RouteKind, String), RouteTargetCandidate>::new();
    let mut truncated = false;
    for fact in snapshot.evidence_kernel.facts() {
        let Some(candidate) = route_target_candidate(snapshot, fact) else {
            continue;
        };
        let key = (candidate.route, candidate.target.clone());
        if !by_route_target.contains_key(&key) && by_route_target.len() == MAX_ROUTE_TARGET_GROUPS {
            truncated = true;
            continue;
        }
        by_route_target
            .entry(key)
            .and_modify(|current| {
                if candidate.sort_key() < current.sort_key() {
                    *current = candidate.clone();
                }
            })
            .or_insert(candidate);
    }

    let mut conflicts = Vec::new();
    let mut groups = BTreeMap::<RouteKind, Vec<RouteTargetCandidate>>::new();
    for ((route, _), candidate) in by_route_target {
        groups.entry(route).or_default().push(candidate);
    }
    for (route, mut candidates) in groups {
        candidates.sort_by_key(RouteTargetCandidate::sort_key);
        for left_index in 0..candidates.len() {
            for right in candidates.iter().skip(left_index + 1) {
                if conflicts.len() == MAX_DOCUMENT_CONFLICTS {
                    return Ok(ConflictBuild {
                        conflicts,
                        truncated: true,
                    });
                }
                let left = &candidates[left_index];
                if left.document == right.document {
                    continue;
                }
                conflicts.push(DocumentConflict::try_new(
                    route,
                    left.to_conflict_side(),
                    right.to_conflict_side(),
                )?);
            }
        }
    }
    Ok(ConflictBuild {
        conflicts,
        truncated,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConflictBuild {
    conflicts: Vec<DocumentConflict>,
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RouteTargetCandidate {
    route: RouteKind,
    document: DocumentId,
    evidence: EvidenceId,
    target: String,
}

impl RouteTargetCandidate {
    fn sort_key(&self) -> (DocumentId, EvidenceId) {
        (self.document, self.evidence)
    }

    fn to_conflict_side(&self) -> DocumentConflictSide {
        DocumentConflictSide {
            document: self.document,
            evidence: self.evidence,
            target: self.target.clone(),
        }
    }
}

fn route_target_candidate(
    snapshot: &RepoSnapshot,
    fact: &EvidenceFact,
) -> Option<RouteTargetCandidate> {
    if fact.kind != EvidenceKind::RouteCandidate {
        return None;
    }
    let route = fact.route?;
    let path = fact.path.as_deref()?;
    let document = snapshot.evidence_kernel_v2.document_id_for_path(path)?;
    let (_, raw_target) = fact.value.rsplit_once(" -> ")?;
    let target = normalized_local_target(raw_target)?;
    Some(RouteTargetCandidate {
        route,
        document,
        evidence: fact.id,
        target,
    })
}

fn normalized_local_target(raw_target: &str) -> Option<String> {
    let target = raw_target.trim().split('#').next()?.trim();
    if target.is_empty()
        || target.starts_with('/')
        || target.starts_with('#')
        || target.contains("://")
        || target.starts_with("mailto:")
    {
        return None;
    }
    Some(target.replace('\\', "/"))
}
