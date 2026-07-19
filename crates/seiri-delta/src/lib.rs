#![forbid(unsafe_code)]

use seiri_core::{
    ArtifactDelta, AuditDeltaReport, AuditSnapshotDigest, CoverageIncompleteReason, CoverageScope,
    CoverageStatus, DeltaCompatibility, DeltaState, DeltaUnknownReason, Digest32, DocumentEvent,
    EvidenceAtom, EvidenceFact, EvidenceFingerprint, EvidenceId, EvidenceIdentityDigest,
    EvidenceOccurrenceDigest, EvidenceProducer, EvidenceStateDigest, ImprovementCandidate,
    MarkdownEvidenceKind, Observation, PortableAuditSnapshot, PortableConflictRecord,
    PortableContentSlotRecord, PortableCoverageRecord, PortableDocumentRecord, PortableFacetRecord,
    PortableObligationRecord, PortableObservationState, PortableRouteRecord, RegressionCandidate,
    RepositoryAnalysis, RouteDelta, SourceDomain, AUDIT_DELTA_SCHEMA_VERSION,
    PORTABLE_AUDIT_SCHEMA_VERSION,
};
use seiri_digest::StableHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

const EVIDENCE_IDENTITY_FINGERPRINT_DOMAIN: &[u8] = b"seiri.evidence.identity.v2";
const EVIDENCE_STATE_FINGERPRINT_DOMAIN: &[u8] = b"seiri.evidence.state.v2";
const EVIDENCE_OCCURRENCE_FINGERPRINT_DOMAIN: &[u8] = b"seiri.evidence.occurrence.v2";

pub fn evidence_fingerprint(
    analysis: &RepositoryAnalysis,
    fact: &EvidenceFact,
) -> Result<EvidenceFingerprint, DeltaError> {
    let kernel = &analysis.evidence_kernel;
    let path = kernel.path_for_fact(fact).unwrap_or_default();
    let mut identity = StableHasher::new(EVIDENCE_IDENTITY_FINGERPRINT_DOMAIN, 7);
    identity.str(1, path);
    identity.u8(2, source_domain_tag(fact.provenance.domain));
    identity.u8(3, producer_tag(fact.provenance.producer));
    hash_atom(&mut identity, fact.atom);
    if let Some(target) = normalized_event_target(analysis, path, fact) {
        identity.str(9, &target);
    }
    let identity = EvidenceIdentityDigest::new(identity.finish());

    let mut state = StableHasher::new(EVIDENCE_STATE_FINGERPRINT_DOMAIN, 4);
    state.digest(1, identity.get());
    state.u8(2, confidence_tag(fact.confidence));
    let classification =
        seiri_core::PathClassification::classify(path, Some(&analysis.repository_scope.graph));
    state.str(3, repository_region_tag(classification.region));
    state.str(4, evidence_usage_tag(classification.usage));
    let state = EvidenceStateDigest::new(state.finish());

    let mut occurrence = StableHasher::new(EVIDENCE_OCCURRENCE_FINGERPRINT_DOMAIN, 6);
    occurrence.digest(1, state.get());
    if let Some(base) = document_base_digest(analysis, path) {
        occurrence.digest(2, base);
    }
    if let Some(span) = fact.provenance.span {
        occurrence.u32(3, span.line.get());
        occurrence.u32(4, span.column.get());
        occurrence.u32(5, span.byte_start.get());
        occurrence.u32(6, span.byte_end.get());
    }
    Ok(EvidenceFingerprint {
        identity,
        state,
        occurrence: EvidenceOccurrenceDigest::new(occurrence.finish()),
    })
}

pub fn evidence_fingerprints_for_ids(
    analysis: &RepositoryAnalysis,
    ids: &[EvidenceId],
) -> Result<Vec<EvidenceFingerprint>, DeltaError> {
    let kernel = &analysis.evidence_kernel;
    let selected = ids.iter().copied().collect::<BTreeSet<_>>();
    let mut fingerprints = kernel
        .facts()
        .iter()
        .filter(|fact| selected.contains(&fact.id))
        .map(|fact| evidence_fingerprint(analysis, fact))
        .collect::<Result<Vec<_>, _>>()?;
    fingerprints.sort_by_key(|fingerprint| {
        (
            fingerprint.identity,
            fingerprint.state,
            fingerprint.occurrence,
        )
    });
    fingerprints.dedup();
    Ok(fingerprints)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaError {
    MissingEvidenceReference,
}

impl Display for DeltaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEvidenceReference => {
                formatter.write_str("portable audit references missing evidence")
            }
        }
    }
}

impl std::error::Error for DeltaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

fn source_domain_tag(value: SourceDomain) -> u8 {
    match value {
        SourceDomain::RepositoryLocal => 0,
        SourceDomain::OrganizationInherited => 1,
        SourceDomain::RemoteRepository => 2,
        SourceDomain::Fixture => 3,
        SourceDomain::Unknown => 4,
    }
}

fn producer_tag(value: EvidenceProducer) -> u8 {
    match value {
        EvidenceProducer::FileWalker => 0,
        EvidenceProducer::Markdown => 1,
    }
}

fn confidence_tag(value: seiri_core::EvidenceConfidence) -> u8 {
    match value {
        seiri_core::EvidenceConfidence::High => 0,
        seiri_core::EvidenceConfidence::Medium => 1,
        seiri_core::EvidenceConfidence::Low => 2,
    }
}

const fn repository_region_tag(value: seiri_core::RepositoryRegion) -> &'static str {
    match value {
        seiri_core::RepositoryRegion::Root => "root",
        seiri_core::RepositoryRegion::WorkspaceMember => "workspace_member",
        seiri_core::RepositoryRegion::Submodule => "submodule",
        seiri_core::RepositoryRegion::Ignored => "ignored",
    }
}

const fn evidence_usage_tag(value: seiri_core::EvidenceUsage) -> &'static str {
    match value {
        seiri_core::EvidenceUsage::Primary => "primary",
        seiri_core::EvidenceUsage::Test => "test",
        seiri_core::EvidenceUsage::Fixture => "fixture",
        seiri_core::EvidenceUsage::Example => "example",
        seiri_core::EvidenceUsage::Generated => "generated",
        seiri_core::EvidenceUsage::Vendored => "vendored",
    }
}

const fn important_file_kind_tag(value: seiri_core::ImportantFileKind) -> &'static str {
    match value {
        seiri_core::ImportantFileKind::Readme => "readme",
        seiri_core::ImportantFileKind::License => "license",
        seiri_core::ImportantFileKind::Contributing => "contributing",
        seiri_core::ImportantFileKind::Security => "security",
        seiri_core::ImportantFileKind::Support => "support",
        seiri_core::ImportantFileKind::IssueTemplate => "issue_template",
        seiri_core::ImportantFileKind::IssueForm => "issue_form",
        seiri_core::ImportantFileKind::PullRequestTemplate => "pull_request_template",
        seiri_core::ImportantFileKind::Changelog => "changelog",
        seiri_core::ImportantFileKind::Codeowners => "codeowners",
        seiri_core::ImportantFileKind::CargoToml => "cargo_toml",
        seiri_core::ImportantFileKind::DocsDirectory => "docs_directory",
        seiri_core::ImportantFileKind::Workflow => "workflow",
        seiri_core::ImportantFileKind::DependencyBot => "dependency_bot",
        seiri_core::ImportantFileKind::SecurityAutomation => "security_automation",
        seiri_core::ImportantFileKind::Gitignore => "gitignore",
        seiri_core::ImportantFileKind::Gitattributes => "gitattributes",
        seiri_core::ImportantFileKind::EditorConfig => "editor_config",
    }
}

const fn readme_presence_tag(value: seiri_core::ReadmePresence) -> &'static str {
    match value {
        seiri_core::ReadmePresence::Present => "present",
        seiri_core::ReadmePresence::Absent => "absent",
    }
}

const fn route_tag(value: seiri_core::RouteKind) -> &'static str {
    value.slug()
}

fn hash_atom(hasher: &mut StableHasher, atom: EvidenceAtom) {
    match atom {
        EvidenceAtom::FilePresent => {
            hasher.u8(4, 0);
        }
        EvidenceAtom::ImportantFile(kind) => {
            hasher.u8(4, 1);
            hasher.str(5, important_file_kind_tag(kind));
        }
        EvidenceAtom::Readme(presence) => {
            hasher.u8(4, 2);
            hasher.str(5, readme_presence_tag(presence));
        }
        EvidenceAtom::Markdown { event, route } => {
            hasher.u8(4, 3);
            hasher.str(
                5,
                match event {
                    MarkdownEvidenceKind::Heading => "heading",
                    MarkdownEvidenceKind::Link => "link",
                    MarkdownEvidenceKind::Badge => "badge",
                    MarkdownEvidenceKind::RouteCandidate => "route_candidate",
                    MarkdownEvidenceKind::VisibleProse => "visible_prose",
                },
            );
            if let Some(route) = route {
                hasher.str(6, route_tag(route));
            }
        }
    }
}

fn normalized_event_target(
    analysis: &RepositoryAnalysis,
    path: &str,
    fact: &EvidenceFact,
) -> Option<String> {
    let span = fact.provenance.span?;
    let scan = analysis
        .document_index
        .entries()
        .iter()
        .find(|entry| entry.path == path)?
        .scan
        .as_ref()?;
    scan.events().iter().find_map(|event| {
        let event_span = event.span()?;
        if event_span.byte_start != span.byte_start.get() as usize
            || event_span.byte_end != span.byte_end.get() as usize
        {
            return None;
        }
        let target = match event {
            DocumentEvent::Link(link) => Some(link.target.as_str()),
            DocumentEvent::RouteCandidate(candidate) => candidate.target.as_deref(),
            DocumentEvent::VisibleProse(_)
            | DocumentEvent::Heading(_)
            | DocumentEvent::Badge(_) => None,
        }?;
        Some(normalize_target(target))
    })
}

fn normalize_target(target: &str) -> String {
    target
        .trim()
        .replace('\\', "/")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn document_base_digest(analysis: &RepositoryAnalysis, path: &str) -> Option<Digest32> {
    analysis
        .document_index
        .entries()
        .iter()
        .find(|entry| entry.path == path)?
        .scan
        .as_ref()
        .map(|scan| scan.base().digest().digest())
}

pub fn portable_snapshot(
    snapshot: &RepositoryAnalysis,
) -> Result<PortableAuditSnapshot, DeltaError> {
    let repository_coverage = coverage(snapshot, CoverageScope::RepositoryFiles);
    let readme_coverage = coverage(snapshot, CoverageScope::RootReadme);
    let route_coverage = combine_coverage(repository_coverage, readme_coverage);

    let mut routes = snapshot
        .route_assessments
        .iter()
        .map(|assessment| {
            let presence = assessment.presence();
            let readme = assessment.readme();
            let mut evidence_ids = assessment
                .evidence()
                .root_structural()
                .iter()
                .chain(assessment.evidence().readme_routing())
                .chain(assessment.evidence().inherited())
                .copied()
                .collect::<Vec<_>>();
            normalize_ids(&mut evidence_ids);
            let readme_routed = readme.routing().is_present();
            let repository_local_targets = readme.target_reachability().repository_local_present();
            let shared_target_conflicts = readme.conflict().shared_target_count();
            let freshness = readme.freshness();
            let observation =
                route_observation(shared_target_conflicts, route_coverage, &evidence_ids);
            let evidence = evidence_fingerprints_for_ids(snapshot, &evidence_ids)?;
            let mut record = PortableRouteRecord {
                route: assessment.route(),
                root_structured: presence.root_structured(),
                inherited: presence.inherited(),
                readme_routed,
                repository_local_targets,
                shared_target_conflicts,
                freshness,
                policy: assessment.policy(),
                missing_pattern: assessment.missing_pattern(),
                observation,
                coverage: route_coverage,
                evidence,
                digest: Digest32::new([0; 32]),
            };
            record.digest = digest_route_record(&record);
            Ok(record)
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    routes.sort_by_key(|record| record.route);

    let markdown_coverage = coverage(snapshot, CoverageScope::MarkdownDocuments);
    let mut content_slots = snapshot
        .route_content
        .assessments
        .iter()
        .map(|assessment| {
            let (observation, evidence_ids, item_coverage) =
                project_observation(&assessment.observation, markdown_coverage);
            let evidence = evidence_fingerprints_for_ids(snapshot, &evidence_ids)?;
            let mut record = PortableContentSlotRecord {
                slot: assessment.slot,
                code: assessment.code.clone(),
                route: assessment.route,
                observation,
                coverage: item_coverage,
                evidence,
                digest: Digest32::new([0; 32]),
            };
            record.digest = digest_content_record(&record);
            Ok(record)
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    content_slots.sort_by_key(|record| record.slot);

    let mut coverage_records = snapshot
        .coverage
        .records()
        .iter()
        .map(|record| {
            let key = coverage_scope_key(snapshot, record.scope)?;
            let mut item = PortableCoverageRecord {
                digest: Digest32::new([0; 32]),
                key,
                status: record.status,
            };
            item.digest = digest_coverage_record(&item);
            Ok(item)
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
            let evidence = evidence_fingerprints_for_ids(snapshot, &evidence_ids)?;
            let mut record = PortableConflictRecord {
                id: conflict.id.clone(),
                route: conflict.route,
                digest: Digest32::new([0; 32]),
                evidence,
            };
            record.digest = digest_conflict_record(&record, conflict.relation);
            Ok(record)
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
            let evidence = evidence_fingerprints_for_ids(snapshot, &evidence_ids)?;
            let mut record = PortableObligationRecord {
                id: obligation.id.clone(),
                route: obligation.route,
                observation,
                coverage: item_coverage,
                digest: Digest32::new([0; 32]),
                evidence,
            };
            record.digest = digest_obligation_record(&record, obligation.facet);
            Ok(record)
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
            let evidence = evidence_fingerprints_for_ids(snapshot, &evidence_ids)?;
            let mut record = PortableFacetRecord {
                facet: assessment.facet,
                observation,
                coverage: item_coverage,
                digest: Digest32::new([0; 32]),
                evidence,
            };
            record.digest = digest_facet_record(&record);
            Ok(record)
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    facets.sort_by_key(|record| record.facet);

    let mut documents = snapshot
        .document_index
        .entries()
        .iter()
        .map(|entry| {
            let item_coverage = entry.status.coverage_status();
            let mut record = PortableDocumentRecord {
                path: entry.path.clone(),
                coverage: item_coverage,
                digest: Digest32::new([0; 32]),
            };
            record.digest = digest_document_record(
                &record,
                entry.role,
                entry.declared_bytes,
                entry.status,
                entry.digest.map(seiri_core::PatchBaseDigest::digest),
                entry.encoding,
            );
            Ok(record)
        })
        .collect::<Result<Vec<_>, DeltaError>>()?;
    documents.sort_by(|left, right| left.path.cmp(&right.path));

    let configuration = digest_configuration(&snapshot.analysis_configuration);
    let all_ids = snapshot
        .evidence_kernel
        .facts()
        .iter()
        .map(|fact| fact.id)
        .collect::<Vec<_>>();
    let evidence_fingerprints = evidence_fingerprints_for_ids(snapshot, &all_ids)?;
    let evidence =
        digest_fingerprint_set(b"seiri.portable-evidence-set.v2", &evidence_fingerprints);
    let digest = AuditSnapshotDigest {
        schema: PORTABLE_AUDIT_SCHEMA_VERSION.to_string(),
        configuration,
        source_session: snapshot.analysis_configuration.source_session_digest,
        evidence,
        routes: digest_record_set(
            b"seiri.portable-route-set.v2",
            routes.iter().map(|item| item.digest),
        ),
        documents: digest_record_set(
            b"seiri.portable-document-set.v2",
            documents.iter().map(|item| item.digest),
        ),
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
                item.evidence.as_slice(),
            )
        }),
        after.content_slots.iter().map(|item| {
            (
                item.code.as_str(),
                item.digest,
                item.coverage,
                item.evidence.as_slice(),
            )
        }),
    );
    let coverage = artifact_deltas(
        before.coverage.iter().map(|item| {
            (
                item.key.as_str(),
                item.digest,
                item.status,
                &[] as &[EvidenceFingerprint],
            )
        }),
        after.coverage.iter().map(|item| {
            (
                item.key.as_str(),
                item.digest,
                item.status,
                &[] as &[EvidenceFingerprint],
            )
        }),
    );
    let conflicts = artifact_deltas_with_domain(
        before
            .conflicts
            .iter()
            .map(|item| (item.id.as_str(), item.digest, item.evidence.as_slice())),
        after
            .conflicts
            .iter()
            .map(|item| (item.id.as_str(), item.digest, item.evidence.as_slice())),
        conflict_coverage(before),
        conflict_coverage(after),
    );
    let obligations = artifact_deltas(
        before.obligations.iter().map(|item| {
            (
                item.id.as_str(),
                item.digest,
                item.coverage,
                item.evidence.as_slice(),
            )
        }),
        after.obligations.iter().map(|item| {
            (
                item.id.as_str(),
                item.digest,
                item.coverage,
                item.evidence.as_slice(),
            )
        }),
    );
    let facets = artifact_deltas(
        before.facets.iter().map(|item| {
            (
                item.facet.slug(),
                item.digest,
                item.coverage,
                item.evidence.as_slice(),
            )
        }),
        after.facets.iter().map(|item| {
            (
                item.facet.slug(),
                item.digest,
                item.coverage,
                item.evidence.as_slice(),
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
                evidence: merged_route_evidence(delta),
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
                    evidence: delta.evidence.clone(),
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
            evidence: merged_route_evidence(delta),
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
                    evidence: delta.evidence.clone(),
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

fn digest_route_record(record: &PortableRouteRecord) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-route.v2", 14);
    hash.str(1, route_tag(record.route))
        .bool(2, record.root_structured)
        .bool(3, record.inherited)
        .bool(4, record.readme_routed)
        .usize(5, record.repository_local_targets)
        .usize(6, record.shared_target_conflicts)
        .str(7, route_freshness_tag(record.freshness))
        .str(8, route_policy_tag(record.policy))
        .bool(9, record.missing_pattern);
    hash_observation_and_coverage(&mut hash, record.observation, record.coverage);
    hash_semantic_fingerprints(&mut hash, &record.evidence);
    hash.finish()
}

fn digest_content_record(record: &PortableContentSlotRecord) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-content-slot.v2", 8);
    hash.u32(1, u32::from(record.slot.0))
        .str(2, &record.code)
        .str(3, route_tag(record.route));
    hash_observation_and_coverage(&mut hash, record.observation, record.coverage);
    hash_semantic_fingerprints(&mut hash, &record.evidence);
    hash.finish()
}

fn digest_coverage_record(record: &PortableCoverageRecord) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-coverage.v2", 2);
    hash.str(1, &record.key);
    hash_coverage(&mut hash, record.status, 2);
    hash.finish()
}

fn digest_conflict_record(
    record: &PortableConflictRecord,
    relation: seiri_core::TargetRelation,
) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-conflict.v2", 5);
    hash.str(1, &record.id)
        .str(2, route_tag(record.route))
        .str(3, target_relation_tag(relation));
    hash_semantic_fingerprints(&mut hash, &record.evidence);
    hash.finish()
}

fn digest_obligation_record(
    record: &PortableObligationRecord,
    facet: seiri_core::RepositoryFacet,
) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-obligation.v2", 8);
    hash.str(1, &record.id)
        .str(2, route_tag(record.route))
        .str(3, facet.slug());
    hash_observation_and_coverage(&mut hash, record.observation, record.coverage);
    hash_semantic_fingerprints(&mut hash, &record.evidence);
    hash.finish()
}

fn digest_facet_record(record: &PortableFacetRecord) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-facet.v2", 5);
    hash.str(1, record.facet.slug());
    hash_observation_and_coverage(&mut hash, record.observation, record.coverage);
    hash_semantic_fingerprints(&mut hash, &record.evidence);
    hash.finish()
}

fn digest_document_record(
    record: &PortableDocumentRecord,
    role: seiri_core::DocumentRole,
    declared_bytes: u64,
    status: seiri_core::DocumentScanStatus,
    base_digest: Option<Digest32>,
    encoding: Option<seiri_core::TextEncoding>,
) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.portable-document.v3", 7);
    hash.str(1, &record.path)
        .str(2, document_role_tag(role))
        .u64(3, declared_bytes)
        .str(4, document_status_tag(status));
    if let Some(base_digest) = base_digest {
        hash.digest(5, base_digest);
    }
    if let Some(encoding) = encoding {
        hash.str(6, text_encoding_tag(encoding));
    }
    hash_coverage(&mut hash, record.coverage, 7);
    hash.finish()
}

fn digest_configuration(configuration: &seiri_core::AnalysisConfiguration) -> Digest32 {
    let mut hash = StableHasher::new(b"seiri.analysis-configuration.v3", 22);
    hash.str(1, &configuration.schema_version)
        .str(2, analysis_scope_tag(configuration.scope))
        .str(3, profile_tag(configuration.profile))
        .usize(4, configuration.budgets.filesystem_max_depth)
        .usize(5, configuration.budgets.filesystem_max_entries)
        .usize(6, configuration.budgets.filesystem_max_directory_entries)
        .usize(7, configuration.budgets.filesystem_max_ignored_records);
    for ignored in &configuration.budgets.filesystem_additional_ignored_names {
        hash.str(8, ignored);
    }
    hash.usize(9, configuration.budgets.document_max_documents)
        .usize(10, configuration.budgets.document_max_total_source_bytes)
        .usize(11, configuration.budgets.document_max_source_bytes)
        .usize(12, configuration.budgets.document_max_events)
        .usize(13, configuration.budgets.document_max_diagnostics)
        .u32(14, configuration.budgets.git_max_refs)
        .u32(15, configuration.budgets.git_max_tags)
        .u32(16, configuration.budgets.git_max_commit_headers)
        .u32(17, configuration.budgets.scope.max_nodes)
        .u64(18, configuration.budgets.scope.max_manifest_bytes)
        .u32(19, configuration.budgets.scope.max_ignored_records)
        .str(20, &configuration.pattern_registry_fingerprint)
        .str(21, visibility_tag(configuration.visibility));
    if let Some(binding) = &configuration.calibration_binding {
        hash.str(22, binding);
    }
    hash.finish()
}

fn hash_observation_and_coverage(
    hash: &mut StableHasher,
    observation: PortableObservationState,
    coverage: CoverageStatus,
) {
    hash.str(30, observation_tag(observation));
    hash_coverage(hash, coverage, 31);
}

fn hash_coverage(hash: &mut StableHasher, coverage: CoverageStatus, tag: u8) {
    match coverage {
        CoverageStatus::Complete => {
            hash.u8(tag, 0);
        }
        CoverageStatus::Partial(reason) => {
            hash.u8(tag, 1)
                .str(tag.saturating_add(1), coverage_reason_tag(reason));
        }
        CoverageStatus::NotRequested => {
            hash.u8(tag, 2);
        }
    }
}

fn hash_fingerprints(hash: &mut StableHasher, evidence: &[EvidenceFingerprint]) {
    hash.usize(40, evidence.len());
    for item in evidence {
        hash.digest(41, item.identity.get())
            .digest(42, item.state.get())
            .digest(43, item.occurrence.get());
    }
}

fn hash_semantic_fingerprints(hash: &mut StableHasher, evidence: &[EvidenceFingerprint]) {
    hash.usize(40, evidence.len());
    for item in evidence {
        hash.digest(41, item.identity.get())
            .digest(42, item.state.get());
    }
}

fn digest_fingerprint_set(domain: &[u8], evidence: &[EvidenceFingerprint]) -> Digest32 {
    let mut hash = StableHasher::new(domain, 4);
    hash_fingerprints(&mut hash, evidence);
    hash.finish()
}

fn digest_record_set(domain: &[u8], records: impl Iterator<Item = Digest32>) -> Digest32 {
    let mut hash = StableHasher::new(domain, 1);
    for record in records {
        hash.digest(1, record);
    }
    hash.finish()
}

const fn route_freshness_tag(value: seiri_core::RouteFreshness) -> &'static str {
    match value {
        seiri_core::RouteFreshness::NotApplicable => "not_applicable",
        seiri_core::RouteFreshness::Current => "current",
        seiri_core::RouteFreshness::Stale => "stale",
        seiri_core::RouteFreshness::Mixed => "mixed",
    }
}

const fn route_policy_tag(value: seiri_core::RoutePolicyBoundary) -> &'static str {
    match value {
        seiri_core::RoutePolicyBoundary::Suggestible => "suggestible",
        seiri_core::RoutePolicyBoundary::MaintainerDecisionRequired => {
            "maintainer_decision_required"
        }
    }
}

const fn target_relation_tag(value: seiri_core::TargetRelation) -> &'static str {
    match value {
        seiri_core::TargetRelation::Equivalent => "equivalent",
        seiri_core::TargetRelation::Refines => "refines",
        seiri_core::TargetRelation::SharedHub => "shared_hub",
        seiri_core::TargetRelation::Competes => "competes",
        seiri_core::TargetRelation::Unknown => "unknown",
    }
}

const fn document_role_tag(value: seiri_core::DocumentRole) -> &'static str {
    match value {
        seiri_core::DocumentRole::RootReadme => "root_readme",
        seiri_core::DocumentRole::Documentation => "documentation",
        seiri_core::DocumentRole::SecurityPolicy => "security_policy",
        seiri_core::DocumentRole::SupportPolicy => "support_policy",
        seiri_core::DocumentRole::ContributionGuide => "contribution_guide",
        seiri_core::DocumentRole::ReleaseNotes => "release_notes",
        seiri_core::DocumentRole::Governance => "governance",
        seiri_core::DocumentRole::GithubConfiguration => "github_configuration",
        seiri_core::DocumentRole::OtherMarkdown => "other_markdown",
    }
}

const fn document_status_tag(value: seiri_core::DocumentScanStatus) -> &'static str {
    match value {
        seiri_core::DocumentScanStatus::Scanned => "scanned",
        seiri_core::DocumentScanStatus::NotMarkdown => "not_markdown",
        seiri_core::DocumentScanStatus::SkippedDocumentBudget => "skipped_document_budget",
        seiri_core::DocumentScanStatus::SkippedByteBudget => "skipped_byte_budget",
        seiri_core::DocumentScanStatus::InvalidUtf8 => "invalid_utf8",
        seiri_core::DocumentScanStatus::ParseFailed => "parse_failed",
        seiri_core::DocumentScanStatus::PermissionDenied => "permission_denied",
    }
}

const fn text_encoding_tag(value: seiri_core::TextEncoding) -> &'static str {
    match value {
        seiri_core::TextEncoding::Utf8 => "utf8",
        seiri_core::TextEncoding::Utf8Bom => "utf8_bom",
        seiri_core::TextEncoding::Unknown => "unknown",
    }
}

const fn analysis_scope_tag(value: seiri_core::AnalysisScope) -> &'static str {
    match value {
        seiri_core::AnalysisScope::Repository => "repository",
        seiri_core::AnalysisScope::Workspace => "workspace",
        seiri_core::AnalysisScope::Subtree => "subtree",
    }
}

const fn profile_tag(value: seiri_core::ProfileKind) -> &'static str {
    value.slug()
}

const fn visibility_tag(value: seiri_core::AnalysisVisibility) -> &'static str {
    match value {
        seiri_core::AnalysisVisibility::Standard => "standard",
        seiri_core::AnalysisVisibility::PublicSyntheticCalibration => {
            "public_synthetic_calibration"
        }
        seiri_core::AnalysisVisibility::LocalPrivateCalibration => "local_private_calibration",
        seiri_core::AnalysisVisibility::RedactedCalibration => "redacted_calibration",
    }
}

const fn observation_tag(value: PortableObservationState) -> &'static str {
    match value {
        PortableObservationState::Present => "present",
        PortableObservationState::Absent => "absent",
        PortableObservationState::Conflict => "conflict",
        PortableObservationState::Unknown => "unknown",
    }
}

const fn coverage_reason_tag(value: CoverageIncompleteReason) -> &'static str {
    match value {
        CoverageIncompleteReason::LimitExceeded => "limit_exceeded",
        CoverageIncompleteReason::InvalidUtf8 => "invalid_utf8",
        CoverageIncompleteReason::ParseFailed => "parse_failed",
        CoverageIncompleteReason::UnsupportedSyntax => "unsupported_syntax",
        CoverageIncompleteReason::PermissionDenied => "permission_denied",
        CoverageIncompleteReason::RateLimited => "rate_limited",
        CoverageIncompleteReason::Unavailable => "unavailable",
    }
}

fn coverage(snapshot: &RepositoryAnalysis, scope: CoverageScope) -> CoverageStatus {
    snapshot
        .coverage
        .record(scope)
        .map_or(CoverageStatus::NotRequested, |record| record.status)
}

fn coverage_scope_key(
    snapshot: &RepositoryAnalysis,
    scope: CoverageScope,
) -> Result<String, DeltaError> {
    Ok(match scope {
        CoverageScope::RepositoryFiles => "repository_files".to_string(),
        CoverageScope::RootReadme => "root_readme".to_string(),
        CoverageScope::MarkdownDocuments => "markdown_documents".to_string(),
        CoverageScope::DocumentRole(role) => {
            format!("document_role:{}", document_role_tag(role))
        }
        CoverageScope::Document(document) => {
            let path = snapshot
                .document_index
                .entries()
                .iter()
                .find(|entry| entry.document_id == Some(document))
                .map(|entry| entry.path.as_str())
                .ok_or(DeltaError::MissingEvidenceReference)?;
            format!("document:{path}")
        }
        CoverageScope::RemoteMetadata => "remote_metadata".to_string(),
    })
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
    shared_target_conflicts: usize,
    coverage: CoverageStatus,
    evidence: &[EvidenceId],
) -> PortableObservationState {
    if coverage != CoverageStatus::Complete && evidence.is_empty() {
        return PortableObservationState::Unknown;
    }
    if shared_target_conflicts > 0 {
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
    } else if (before.configuration.visibility
        == seiri_core::AnalysisVisibility::LocalPrivateCalibration
        || after.configuration.visibility
            == seiri_core::AnalysisVisibility::LocalPrivateCalibration)
        && (before.configuration.calibration_binding.is_none()
            || after.configuration.calibration_binding.is_none())
    {
        DeltaCompatibility::Unknown(DeltaUnknownReason::UnknownPrivateBinding)
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
    I: IntoIterator<Item = (&'a str, Digest32, CoverageStatus, &'a [EvidenceFingerprint])>,
    J: IntoIterator<Item = (&'a str, Digest32, CoverageStatus, &'a [EvidenceFingerprint])>,
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
    I: IntoIterator<Item = (&'a str, Digest32, &'a [EvidenceFingerprint])>,
    J: IntoIterator<Item = (&'a str, Digest32, &'a [EvidenceFingerprint])>,
{
    artifact_deltas(
        before
            .into_iter()
            .map(|(k, d, e)| (k, d, before_coverage, e)),
        after.into_iter().map(|(k, d, e)| (k, d, after_coverage, e)),
    )
}

fn artifact_maps(
    left: BTreeMap<String, (Digest32, CoverageStatus, Vec<EvidenceFingerprint>)>,
    right: BTreeMap<String, (Digest32, CoverageStatus, Vec<EvidenceFingerprint>)>,
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
            let mut evidence = before
                .into_iter()
                .flat_map(|item| item.2.iter())
                .chain(after.into_iter().flat_map(|item| item.2.iter()))
                .copied()
                .collect::<Vec<_>>();
            normalize_fingerprints(&mut evidence);
            ArtifactDelta {
                key,
                state: state_for(before.map(|x| (x.0, x.1)), after.map(|x| (x.0, x.1))),
                before: before.map(|x| x.0),
                after: after.map(|x| x.0),
                before_coverage,
                after_coverage,
                evidence,
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
        (Some(left), Some(right)) if route_signal_change(left, right) == (true, false) => {
            DeltaState::Removed
        }
        (Some(left), Some(right)) if route_signal_change(left, right) == (false, true) => {
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

fn route_signal_change(before: &PortableRouteRecord, after: &PortableRouteRecord) -> (bool, bool) {
    let before_signals = [
        before.root_structured,
        before.inherited,
        before.readme_routed,
        before.repository_local_targets > 0,
    ];
    let after_signals = [
        after.root_structured,
        after.inherited,
        after.readme_routed,
        after.repository_local_targets > 0,
    ];
    before_signals
        .into_iter()
        .zip(after_signals)
        .fold((false, false), |(lost, gained), (left, right)| {
            (lost || (left && !right), gained || (!left && right))
        })
}

fn merged_route_evidence(delta: &RouteDelta) -> Vec<EvidenceFingerprint> {
    let mut ids = delta
        .before
        .iter()
        .flat_map(|x| x.evidence.iter())
        .chain(delta.after.iter().flat_map(|x| x.evidence.iter()))
        .copied()
        .collect::<Vec<_>>();
    normalize_fingerprints(&mut ids);
    ids
}

fn normalize_fingerprints(values: &mut Vec<EvidenceFingerprint>) {
    values.sort_unstable();
    values.dedup();
}

fn normalize_ids(ids: &mut Vec<EvidenceId>) {
    ids.sort_unstable();
    ids.dedup();
}

fn delta_boundary() -> String {
    "Audit delta compares canonical observations only when schema, scope, and configuration match. Partial or missing coverage yields Unknown and is never promoted to a regression. SHA-256 values are deterministic comparison guards, not signatures, authenticity evidence, security proof, or correctness proof.".to_string()
}
