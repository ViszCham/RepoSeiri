use seiri_core::{
    ArtifactDelta, AuditDeltaReport, AuditSnapshotDigest, CoverageIncompleteReason, CoverageScope,
    CoverageStatus, DeltaCompatibility, DeltaState, DeltaUnknownReason, Digest32, EvidenceId,
    ImprovementCandidate, Observation, PortableAuditSnapshot, PortableConflictRecord,
    PortableContentSlotRecord, PortableCoverageRecord, PortableDocumentRecord, PortableFacetRecord,
    PortableObligationRecord, PortableObservationState, PortableRouteRecord, RegressionCandidate,
    RepoSnapshot, RouteDelta, RouteState, AUDIT_DELTA_SCHEMA_VERSION,
    PORTABLE_AUDIT_SCHEMA_VERSION,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum DeltaError {
    Serialize(serde_json::Error),
}

impl Display for DeltaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(error) => write!(
                formatter,
                "failed to serialize canonical audit data: {error}"
            ),
        }
    }
}

impl std::error::Error for DeltaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for DeltaError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialize(value)
    }
}

pub fn portable_snapshot(snapshot: &RepoSnapshot) -> Result<PortableAuditSnapshot, DeltaError> {
    let repository_coverage = coverage(snapshot, CoverageScope::RepositoryFiles);
    let readme_coverage = coverage(snapshot, CoverageScope::RootReadme);
    let route_coverage = combine_coverage(repository_coverage, readme_coverage);

    let mut routes = snapshot
        .route_assessments
        .iter()
        .map(|assessment| {
            let legacy_state = assessment.legacy_projection().state;
            let mut evidence_ids = assessment.legacy_evidence_ids();
            normalize_ids(&mut evidence_ids);
            let observation = route_observation(legacy_state, route_coverage, &evidence_ids);
            let digest = digest_json(&(
                assessment.route(),
                legacy_state,
                observation,
                route_coverage,
                &evidence_ids,
            ))?;
            Ok(PortableRouteRecord {
                route: assessment.route(),
                legacy_state,
                observation,
                coverage: route_coverage,
                evidence_ids,
                digest,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    routes.sort_by_key(|record| record.route);

    let markdown_coverage = coverage(snapshot, CoverageScope::MarkdownDocuments);
    let mut content_slots = snapshot
        .route_content_v2
        .assessments
        .iter()
        .map(|assessment| {
            let (observation, evidence_ids, item_coverage) =
                project_observation(&assessment.observation, markdown_coverage);
            let digest = digest_json(&(
                assessment.slot,
                &assessment.code,
                assessment.route,
                observation,
                item_coverage,
                &evidence_ids,
            ))?;
            Ok(PortableContentSlotRecord {
                slot: assessment.slot,
                code: assessment.code.clone(),
                route: assessment.route,
                observation,
                coverage: item_coverage,
                evidence_ids,
                digest,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    content_slots.sort_by_key(|record| record.slot);

    let mut coverage_records = snapshot
        .coverage
        .records()
        .iter()
        .map(|record| {
            let key = serde_json::to_string(&record.scope)?;
            Ok(PortableCoverageRecord {
                digest: digest_json(&(&key, record.status))?,
                key,
                status: record.status,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    coverage_records.sort_by(|left, right| left.key.cmp(&right.key));

    let mut conflicts = snapshot
        .document_consistency
        .conflicts
        .iter()
        .map(|conflict| {
            let mut evidence_ids = vec![conflict.left.evidence, conflict.right.evidence];
            normalize_ids(&mut evidence_ids);
            Ok(PortableConflictRecord {
                id: conflict.id.clone(),
                route: conflict.route,
                digest: digest_json(&(
                    &conflict.id,
                    conflict.route,
                    conflict.relation,
                    &evidence_ids,
                ))?,
                evidence_ids,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    conflicts.sort_by(|left, right| left.id.cmp(&right.id));

    let mut obligations = snapshot
        .document_consistency
        .obligations
        .iter()
        .map(|obligation| {
            let (observation, mut evidence_ids, item_coverage) =
                project_observation(&obligation.observation, repository_coverage);
            evidence_ids.extend_from_slice(obligation.reason.as_slice());
            normalize_ids(&mut evidence_ids);
            Ok(PortableObligationRecord {
                id: obligation.id.clone(),
                route: obligation.route,
                observation,
                coverage: item_coverage,
                digest: digest_json(&(
                    &obligation.id,
                    obligation.facet,
                    obligation.route,
                    observation,
                    item_coverage,
                    &evidence_ids,
                ))?,
                evidence_ids,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    obligations.sort_by(|left, right| left.id.cmp(&right.id));

    let mut facets = snapshot
        .facets
        .facets
        .iter()
        .map(|assessment| {
            let (observation, evidence_ids, item_coverage) =
                project_observation(&assessment.observation, repository_coverage);
            Ok(PortableFacetRecord {
                facet: assessment.facet,
                observation,
                coverage: item_coverage,
                digest: digest_json(&(
                    assessment.facet,
                    observation,
                    item_coverage,
                    &evidence_ids,
                ))?,
                evidence_ids,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    facets.sort_by_key(|record| record.facet);

    let mut documents = snapshot
        .document_index
        .entries()
        .iter()
        .map(|entry| {
            let item_coverage = entry.status.coverage_status();
            Ok(PortableDocumentRecord {
                document: entry.document_id,
                path: entry.path.clone(),
                coverage: item_coverage,
                digest: digest_json(&(
                    &entry.path,
                    entry.role,
                    entry.declared_bytes,
                    entry.status,
                    entry.digest,
                    entry.encoding,
                ))?,
            })
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    documents.sort_by(|left, right| left.path.cmp(&right.path));

    let configuration = digest_json(&snapshot.analysis_configuration)?;
    let evidence = digest_json(snapshot.evidence_kernel_v2.facts())?;
    let digest = AuditSnapshotDigest {
        schema: PORTABLE_AUDIT_SCHEMA_VERSION.to_string(),
        configuration,
        evidence,
        routes: digest_json(&routes)?,
        documents: digest_json(&documents)?,
    };

    Ok(PortableAuditSnapshot {
        schema_version: PORTABLE_AUDIT_SCHEMA_VERSION.to_string(),
        configuration: snapshot.analysis_configuration.clone(),
        digest,
        routes,
        content_slots,
        coverage: coverage_records,
        conflicts,
        obligations,
        facets,
        documents,
        boundary: "This portable snapshot contains canonical typed observations, redacted configuration identity, and deterministic SHA-256 comparison guards. It excludes source text and private calibration values. Digests are not signatures, authenticity evidence, security proof, or correctness proof.".to_string(),
    })
}

pub fn compare(before: &PortableAuditSnapshot, after: &PortableAuditSnapshot) -> AuditDeltaReport {
    let compatibility = compatibility(before, after);
    if compatibility != DeltaCompatibility::Comparable {
        return empty_report(before, after, compatibility);
    }

    let routes = route_deltas(before, after);
    let content_slots = artifact_deltas(
        before.content_slots.iter().map(|item| {
            (
                item.code.as_str(),
                item.digest,
                item.coverage,
                item.evidence_ids.as_slice(),
            )
        }),
        after.content_slots.iter().map(|item| {
            (
                item.code.as_str(),
                item.digest,
                item.coverage,
                item.evidence_ids.as_slice(),
            )
        }),
    );
    let coverage = artifact_deltas(
        before.coverage.iter().map(|item| {
            (
                item.key.as_str(),
                item.digest,
                item.status,
                &[] as &[EvidenceId],
            )
        }),
        after.coverage.iter().map(|item| {
            (
                item.key.as_str(),
                item.digest,
                item.status,
                &[] as &[EvidenceId],
            )
        }),
    );
    let conflicts = artifact_deltas_with_domain(
        before
            .conflicts
            .iter()
            .map(|item| (item.id.as_str(), item.digest, item.evidence_ids.as_slice())),
        after
            .conflicts
            .iter()
            .map(|item| (item.id.as_str(), item.digest, item.evidence_ids.as_slice())),
        conflict_coverage(before),
        conflict_coverage(after),
    );
    let obligations = artifact_deltas(
        before.obligations.iter().map(|item| {
            (
                item.id.as_str(),
                item.digest,
                item.coverage,
                item.evidence_ids.as_slice(),
            )
        }),
        after.obligations.iter().map(|item| {
            (
                item.id.as_str(),
                item.digest,
                item.coverage,
                item.evidence_ids.as_slice(),
            )
        }),
    );
    let facets = artifact_deltas(
        before.facets.iter().map(|item| {
            (
                item.facet.slug(),
                item.digest,
                item.coverage,
                item.evidence_ids.as_slice(),
            )
        }),
        after.facets.iter().map(|item| {
            (
                item.facet.slug(),
                item.digest,
                item.coverage,
                item.evidence_ids.as_slice(),
            )
        }),
    );

    let mut regressions = Vec::new();
    for delta in &routes {
        if route_is_regression(delta)
            && delta
                .before
                .as_ref()
                .is_some_and(|item| item.coverage == CoverageStatus::Complete)
            && delta
                .after
                .as_ref()
                .is_some_and(|item| item.coverage == CoverageStatus::Complete)
        {
            regressions.push(RegressionCandidate {
                domain: "route".to_string(),
                key: format!("{:?}", delta.route),
                state: delta.state,
                evidence_ids: merged_route_evidence(delta),
            });
        }
    }
    for (domain, deltas) in [
        ("content_slot", &content_slots),
        ("coverage", &coverage),
        ("conflict", &conflicts),
        ("obligation", &obligations),
        ("facet", &facets),
    ] {
        regressions.extend(
            deltas
                .iter()
                .filter(|delta| {
                    delta.state == DeltaState::Removed
                        && delta.before_coverage == CoverageStatus::Complete
                        && delta.after_coverage == CoverageStatus::Complete
                })
                .map(|delta| RegressionCandidate {
                    domain: domain.to_string(),
                    key: delta.key.clone(),
                    state: delta.state,
                    evidence_ids: delta.evidence_ids.clone(),
                }),
        );
    }

    let mut improvements = routes
        .iter()
        .filter(|delta| {
            delta.state == DeltaState::Added
                && delta
                    .before
                    .as_ref()
                    .is_some_and(|item| item.coverage == CoverageStatus::Complete)
                && delta
                    .after
                    .as_ref()
                    .is_some_and(|item| item.coverage == CoverageStatus::Complete)
        })
        .map(|delta| ImprovementCandidate {
            domain: "route".to_string(),
            key: format!("{:?}", delta.route),
            state: delta.state,
            evidence_ids: merged_route_evidence(delta),
        })
        .collect::<Vec<_>>();
    for (domain, deltas) in [
        ("content_slot", &content_slots),
        ("coverage", &coverage),
        ("conflict", &conflicts),
        ("obligation", &obligations),
        ("facet", &facets),
    ] {
        improvements.extend(
            deltas
                .iter()
                .filter(|delta| {
                    delta.state == DeltaState::Added
                        && delta.before_coverage == CoverageStatus::Complete
                        && delta.after_coverage == CoverageStatus::Complete
                })
                .map(|delta| ImprovementCandidate {
                    domain: domain.to_string(),
                    key: delta.key.clone(),
                    state: delta.state,
                    evidence_ids: delta.evidence_ids.clone(),
                }),
        );
    }

    AuditDeltaReport {
        schema_version: AUDIT_DELTA_SCHEMA_VERSION.to_string(),
        compatibility,
        before: before.digest.clone(),
        after: after.digest.clone(),
        routes,
        content_slots,
        coverage,
        conflicts,
        obligations,
        facets,
        regressions,
        improvements,
        boundary: delta_boundary(),
    }
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> Result<Digest32, DeltaError> {
    let bytes = serde_json::to_vec(value)?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    Ok(Digest32::new(digest))
}

fn coverage(snapshot: &RepoSnapshot, scope: CoverageScope) -> CoverageStatus {
    snapshot
        .coverage
        .record(scope)
        .map_or(CoverageStatus::NotRequested, |record| record.status)
}

fn combine_coverage(left: CoverageStatus, right: CoverageStatus) -> CoverageStatus {
    match (left, right) {
        (CoverageStatus::Complete, CoverageStatus::Complete) => CoverageStatus::Complete,
        (CoverageStatus::Partial(reason), _) | (_, CoverageStatus::Partial(reason)) => {
            CoverageStatus::Partial(reason)
        }
        _ => CoverageStatus::NotRequested,
    }
}

fn route_observation(
    state: RouteState,
    coverage: CoverageStatus,
    evidence: &[EvidenceId],
) -> PortableObservationState {
    if coverage != CoverageStatus::Complete && evidence.is_empty() {
        return PortableObservationState::Unknown;
    }
    if state == RouteState::Conflicting {
        PortableObservationState::Conflict
    } else if !evidence.is_empty() {
        PortableObservationState::Present
    } else if coverage == CoverageStatus::Complete {
        PortableObservationState::Absent
    } else {
        PortableObservationState::Unknown
    }
}

fn project_observation<T>(
    observation: &Observation<T>,
    fallback: CoverageStatus,
) -> (PortableObservationState, Vec<EvidenceId>, CoverageStatus) {
    match observation {
        Observation::Present { evidence, .. } => (
            PortableObservationState::Present,
            evidence.as_slice().to_vec(),
            fallback,
        ),
        Observation::Conflict { alternatives } => (
            PortableObservationState::Conflict,
            alternatives.as_slice().to_vec(),
            fallback,
        ),
        Observation::Absent { .. } => (
            PortableObservationState::Absent,
            Vec::new(),
            CoverageStatus::Complete,
        ),
        Observation::Unknown(reason) => (
            PortableObservationState::Unknown,
            Vec::new(),
            coverage_for_unknown(*reason),
        ),
    }
}

fn coverage_for_unknown(reason: seiri_core::UnknownReason) -> CoverageStatus {
    use seiri_core::UnknownReason as Reason;
    match reason {
        Reason::NotRequested => CoverageStatus::NotRequested,
        Reason::LimitExceeded => CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded),
        Reason::InvalidUtf8 => CoverageStatus::Partial(CoverageIncompleteReason::InvalidUtf8),
        Reason::ParseFailed => CoverageStatus::Partial(CoverageIncompleteReason::ParseFailed),
        Reason::UnsupportedSyntax => {
            CoverageStatus::Partial(CoverageIncompleteReason::UnsupportedSyntax)
        }
        Reason::PermissionDenied => {
            CoverageStatus::Partial(CoverageIncompleteReason::PermissionDenied)
        }
        Reason::RateLimited => CoverageStatus::Partial(CoverageIncompleteReason::RateLimited),
        Reason::Unavailable => CoverageStatus::Partial(CoverageIncompleteReason::Unavailable),
    }
}

fn compatibility(
    before: &PortableAuditSnapshot,
    after: &PortableAuditSnapshot,
) -> DeltaCompatibility {
    if before.schema_version != after.schema_version || before.digest.schema != after.digest.schema
    {
        DeltaCompatibility::Unknown(DeltaUnknownReason::SchemaMismatch)
    } else if before.configuration.scope != after.configuration.scope {
        DeltaCompatibility::Unknown(DeltaUnknownReason::ScopeMismatch)
    } else if before.digest.configuration != after.digest.configuration {
        DeltaCompatibility::Unknown(DeltaUnknownReason::ConfigurationMismatch)
    } else {
        DeltaCompatibility::Comparable
    }
}

fn empty_report(
    before: &PortableAuditSnapshot,
    after: &PortableAuditSnapshot,
    compatibility: DeltaCompatibility,
) -> AuditDeltaReport {
    AuditDeltaReport {
        schema_version: AUDIT_DELTA_SCHEMA_VERSION.to_string(),
        compatibility,
        before: before.digest.clone(),
        after: after.digest.clone(),
        routes: Vec::new(),
        content_slots: Vec::new(),
        coverage: Vec::new(),
        conflicts: Vec::new(),
        obligations: Vec::new(),
        facets: Vec::new(),
        regressions: Vec::new(),
        improvements: Vec::new(),
        boundary: delta_boundary(),
    }
}

fn route_deltas(before: &PortableAuditSnapshot, after: &PortableAuditSnapshot) -> Vec<RouteDelta> {
    let before_map = before
        .routes
        .iter()
        .map(|item| (item.route, item))
        .collect::<BTreeMap<_, _>>();
    let after_map = after
        .routes
        .iter()
        .map(|item| (item.route, item))
        .collect::<BTreeMap<_, _>>();
    before_map
        .keys()
        .chain(after_map.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|route| {
            let left = before_map.get(&route).copied();
            let right = after_map.get(&route).copied();
            RouteDelta {
                route,
                state: route_state_for(left, right),
                before: left.cloned(),
                after: right.cloned(),
            }
        })
        .collect()
}

fn artifact_deltas<'a, I, J>(before: I, after: J) -> Vec<ArtifactDelta>
where
    I: IntoIterator<Item = (&'a str, Digest32, CoverageStatus, &'a [EvidenceId])>,
    J: IntoIterator<Item = (&'a str, Digest32, CoverageStatus, &'a [EvidenceId])>,
{
    let left = before
        .into_iter()
        .map(|(key, digest, coverage, evidence)| {
            (key.to_string(), (digest, coverage, evidence.to_vec()))
        })
        .collect::<BTreeMap<_, _>>();
    let right = after
        .into_iter()
        .map(|(key, digest, coverage, evidence)| {
            (key.to_string(), (digest, coverage, evidence.to_vec()))
        })
        .collect::<BTreeMap<_, _>>();
    artifact_maps(left, right)
}

fn artifact_deltas_with_domain<'a, I, J>(
    before: I,
    after: J,
    before_coverage: CoverageStatus,
    after_coverage: CoverageStatus,
) -> Vec<ArtifactDelta>
where
    I: IntoIterator<Item = (&'a str, Digest32, &'a [EvidenceId])>,
    J: IntoIterator<Item = (&'a str, Digest32, &'a [EvidenceId])>,
{
    artifact_deltas(
        before
            .into_iter()
            .map(|(k, d, e)| (k, d, before_coverage, e)),
        after.into_iter().map(|(k, d, e)| (k, d, after_coverage, e)),
    )
}

fn artifact_maps(
    left: BTreeMap<String, (Digest32, CoverageStatus, Vec<EvidenceId>)>,
    right: BTreeMap<String, (Digest32, CoverageStatus, Vec<EvidenceId>)>,
) -> Vec<ArtifactDelta> {
    left.keys()
        .chain(right.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|key| {
            let before = left.get(&key);
            let after = right.get(&key);
            let before_coverage = before.map_or(CoverageStatus::NotRequested, |item| item.1);
            let after_coverage = after.map_or(CoverageStatus::NotRequested, |item| item.1);
            let mut evidence_ids = before
                .into_iter()
                .flat_map(|item| item.2.iter())
                .chain(after.into_iter().flat_map(|item| item.2.iter()))
                .copied()
                .collect::<Vec<_>>();
            normalize_ids(&mut evidence_ids);
            ArtifactDelta {
                key,
                state: state_for(before.map(|x| (x.0, x.1)), after.map(|x| (x.0, x.1))),
                before: before.map(|x| x.0),
                after: after.map(|x| x.0),
                before_coverage,
                after_coverage,
                evidence_ids,
            }
        })
        .collect()
}

fn state_for(
    before: Option<(Digest32, CoverageStatus)>,
    after: Option<(Digest32, CoverageStatus)>,
) -> DeltaState {
    match (before, after) {
        (Some((_, left_cov)), Some((_, right_cov)))
            if left_cov != CoverageStatus::Complete || right_cov != CoverageStatus::Complete =>
        {
            DeltaState::Unknown
        }
        (Some((left, _)), Some((right, _))) if left == right => DeltaState::Unchanged,
        (Some(_), Some(_)) => DeltaState::Changed,
        (None, Some((_, CoverageStatus::Complete))) => DeltaState::Added,
        (Some((_, CoverageStatus::Complete)), None) => DeltaState::Removed,
        _ => DeltaState::Unknown,
    }
}

fn conflict_coverage(snapshot: &PortableAuditSnapshot) -> CoverageStatus {
    snapshot
        .coverage
        .iter()
        .find(|item| item.key.contains("markdown_documents"))
        .map_or(CoverageStatus::NotRequested, |item| item.status)
}

fn route_state_for(
    before: Option<&PortableRouteRecord>,
    after: Option<&PortableRouteRecord>,
) -> DeltaState {
    match (before, after) {
        (Some(left), Some(right))
            if left.coverage != CoverageStatus::Complete
                || right.coverage != CoverageStatus::Complete =>
        {
            DeltaState::Unknown
        }
        (Some(left), Some(right)) if left.digest == right.digest => DeltaState::Unchanged,
        (Some(left), Some(right))
            if route_is_exposed(left.legacy_state) && !route_is_exposed(right.legacy_state) =>
        {
            DeltaState::Removed
        }
        (Some(left), Some(right))
            if !route_is_exposed(left.legacy_state) && route_is_exposed(right.legacy_state) =>
        {
            DeltaState::Added
        }
        (Some(left), Some(right))
            if left.observation == PortableObservationState::Absent
                && right.observation == PortableObservationState::Present =>
        {
            DeltaState::Added
        }
        (Some(left), Some(right))
            if left.observation == PortableObservationState::Present
                && right.observation == PortableObservationState::Absent =>
        {
            DeltaState::Removed
        }
        (Some(_), Some(_)) => DeltaState::Changed,
        (None, Some(right)) if right.coverage == CoverageStatus::Complete => DeltaState::Added,
        (Some(left), None) if left.coverage == CoverageStatus::Complete => DeltaState::Removed,
        _ => DeltaState::Unknown,
    }
}

fn route_is_regression(delta: &RouteDelta) -> bool {
    delta.state == DeltaState::Removed
        || (delta.state == DeltaState::Changed
            && delta
                .before
                .as_ref()
                .is_some_and(|item| item.observation == PortableObservationState::Present)
            && delta
                .after
                .as_ref()
                .is_some_and(|item| item.observation == PortableObservationState::Conflict))
}

fn route_is_exposed(state: RouteState) -> bool {
    matches!(
        state,
        RouteState::Routed
            | RouteState::Verified
            | RouteState::Stale
            | RouteState::Overloaded
            | RouteState::Conflicting
    )
}

fn merged_route_evidence(delta: &RouteDelta) -> Vec<EvidenceId> {
    let mut ids = delta
        .before
        .iter()
        .flat_map(|x| x.evidence_ids.iter())
        .chain(delta.after.iter().flat_map(|x| x.evidence_ids.iter()))
        .copied()
        .collect::<Vec<_>>();
    normalize_ids(&mut ids);
    ids
}

fn normalize_ids(ids: &mut Vec<EvidenceId>) {
    ids.sort_unstable();
    ids.dedup();
}

fn delta_boundary() -> String {
    "Audit delta compares canonical observations only when schema, scope, and configuration match. Partial or missing coverage yields Unknown and is never promoted to a regression. SHA-256 values are deterministic comparison guards, not signatures, authenticity evidence, security proof, or correctness proof.".to_string()
}
