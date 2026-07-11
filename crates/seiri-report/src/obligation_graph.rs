use seiri_core::{
    classify_target_relation, ConditionalObligation, CoverageIncompleteReason, CoverageScope,
    CoverageStatus, DocumentConflict, DocumentConflictSide, DocumentConsistencyError,
    DocumentConsistencyReport, DocumentEvent, DocumentTargetRelation, EvidenceAtom, EvidenceSet,
    MarkdownEvidenceKind, Observation, RepositoryAnalysis, RepositoryFacet, RouteKind,
    RouteTargetRef, RouteTargetRole, SourceSpan, TargetRelation,
};
use std::collections::BTreeMap;

const MAX_DOCUMENT_CONFLICTS: usize = 64;
const MAX_ROUTE_TARGETS: usize = 128;
const MAX_ROUTE_TARGET_RELATIONS: usize = 256;

pub(crate) struct RouteTargetBuild {
    pub(crate) targets: Vec<RouteTargetRef>,
    pub(crate) truncated: bool,
}

pub(crate) fn build_route_targets(snapshot: &RepositoryAnalysis) -> RouteTargetBuild {
    let mut targets = Vec::new();
    let mut truncated = false;
    for entry in snapshot.document_index.scanned_documents() {
        let Some(document) = entry.scan.as_ref() else {
            continue;
        };
        let Some(document_id) = entry.document_id else {
            continue;
        };
        if is_fixture_document(&entry.path) {
            continue;
        }
        for event in document.events() {
            let DocumentEvent::RouteCandidate(candidate) = event else {
                continue;
            };
            let (Some(raw_target), Some(span)) = (candidate.target.as_deref(), candidate.span)
            else {
                continue;
            };
            let Some(normalized_target) = normalized_local_target(&entry.path, raw_target) else {
                continue;
            };
            let Some(evidence) =
                evidence_for_route_candidate(snapshot, &entry.path, candidate.route, span)
            else {
                continue;
            };
            if targets.len() == MAX_ROUTE_TARGETS {
                truncated = true;
                continue;
            }
            targets.push(RouteTargetRef {
                route: candidate.route,
                document: document_id,
                evidence,
                span,
                role: classify_target_role(candidate.route, &candidate.text, &normalized_target),
                normalized_target,
            });
        }
    }
    targets.sort_by(|left, right| {
        left.route
            .cmp(&right.route)
            .then_with(|| left.normalized_target.cmp(&right.normalized_target))
            .then_with(|| left.document.cmp(&right.document))
            .then_with(|| left.evidence.cmp(&right.evidence))
    });
    RouteTargetBuild { targets, truncated }
}

pub(crate) fn build_document_consistency_report(
    snapshot: &RepositoryAnalysis,
    route_targets_truncated: bool,
) -> Result<DocumentConsistencyReport, DocumentConsistencyError> {
    let mut obligations = build_conditional_obligations(snapshot);
    obligations.sort_by(|left, right| left.id.cmp(&right.id));

    let conflict_build = build_document_relations(snapshot, route_targets_truncated)?;
    let mut relations = conflict_build.relations;
    relations.sort_by(|left, right| left.id.cmp(&right.id));
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
    DocumentConsistencyReport::try_new(obligations, relations, conflicts, conflict_coverage)
}

fn build_conditional_obligations(snapshot: &RepositoryAnalysis) -> Vec<ConditionalObligation> {
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

fn route_observation(snapshot: &RepositoryAnalysis, route: RouteKind) -> Observation<()> {
    let evidence = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == route)
        .map_or_else(Vec::new, |assessment| assessment.summary_evidence_ids());
    if evidence.is_empty() {
        snapshot
            .coverage
            .observe_absence(CoverageScope::RepositoryFiles)
    } else {
        Observation::present((), evidence)
            .expect("route state evidence ids are non-empty after collection")
    }
}

fn build_document_relations(
    snapshot: &RepositoryAnalysis,
    route_targets_truncated: bool,
) -> Result<ConflictBuild, DocumentConsistencyError> {
    let mut truncated = route_targets_truncated;
    let mut relations = Vec::new();
    let mut conflicts = Vec::new();
    let mut groups = BTreeMap::<RouteKind, Vec<&RouteTargetRef>>::new();
    for candidate in &snapshot.route_targets {
        let route = candidate.route;
        groups.entry(route).or_default().push(candidate);
    }
    for (route, mut candidates) in groups {
        candidates.sort_by_key(|candidate| (candidate.document, candidate.evidence));
        for left_index in 0..candidates.len() {
            for right in candidates.iter().skip(left_index + 1) {
                let left = &candidates[left_index];
                if left.document == right.document {
                    continue;
                }
                if relations.len() == MAX_ROUTE_TARGET_RELATIONS {
                    truncated = true;
                    continue;
                }
                let relation = classify_target_relation(left, right);
                let left_side = conflict_side(left);
                let right_side = conflict_side(right);
                relations.push(DocumentTargetRelation::new(
                    route,
                    left_side.clone(),
                    right_side.clone(),
                    relation,
                ));
                if relation == TargetRelation::Competes {
                    if conflicts.len() == MAX_DOCUMENT_CONFLICTS {
                        truncated = true;
                        continue;
                    }
                    conflicts.push(DocumentConflict::try_new(route, left_side, right_side)?);
                }
            }
        }
    }
    Ok(ConflictBuild {
        relations,
        conflicts,
        truncated,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConflictBuild {
    relations: Vec<DocumentTargetRelation>,
    conflicts: Vec<DocumentConflict>,
    truncated: bool,
}

fn conflict_side(candidate: &RouteTargetRef) -> DocumentConflictSide {
    DocumentConflictSide {
        document: candidate.document,
        evidence: candidate.evidence,
        target: candidate.normalized_target.clone(),
        role: candidate.role,
        span: Some(candidate.span),
    }
}

fn evidence_for_route_candidate(
    snapshot: &RepositoryAnalysis,
    path: &str,
    route: RouteKind,
    span: SourceSpan,
) -> Option<seiri_core::EvidenceId> {
    snapshot.evidence_kernel.facts().iter().find_map(|fact| {
        let matches = snapshot.evidence_kernel.path_for_fact(fact) == Some(path)
            && matches!(
                fact.atom,
                EvidenceAtom::Markdown {
                    event: MarkdownEvidenceKind::RouteCandidate,
                    route: Some(actual),
                } if actual == route
            )
            && fact
                .provenance
                .span
                .is_some_and(|actual| span_matches(actual, span));
        matches.then_some(fact.id)
    })
}

fn span_matches(actual: seiri_core::EvidenceSourceSpan, expected: SourceSpan) -> bool {
    actual.line.get() == u32::try_from(expected.line).unwrap_or(u32::MAX)
        && actual.column.get() == u32::try_from(expected.column).unwrap_or(u32::MAX)
        && actual.byte_start.get() == u32::try_from(expected.byte_start).unwrap_or(u32::MAX)
        && actual.byte_end.get() == u32::try_from(expected.byte_end).unwrap_or(u32::MAX)
}

fn normalized_local_target(document_path: &str, raw_target: &str) -> Option<String> {
    let target = raw_target
        .trim()
        .split(['#', '?'])
        .next()?
        .trim()
        .replace('\\', "/");
    if target.is_empty()
        || target.starts_with('/')
        || target.starts_with('#')
        || target.contains("://")
        || target.starts_with("mailto:")
        || target
            .split('/')
            .next()
            .is_some_and(|part| part.contains(':'))
    {
        return None;
    }

    let mut components = document_path
        .replace('\\', "/")
        .split('/')
        .map(str::to_string)
        .collect::<Vec<_>>();
    components.pop();
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            value => components.push(value.to_string()),
        }
    }
    (!components.is_empty()).then(|| components.join("/"))
}

fn classify_target_role(route: RouteKind, label: &str, normalized_target: &str) -> RouteTargetRole {
    let target = normalized_target.to_ascii_lowercase();
    let label = label.trim().to_ascii_lowercase();
    let file_name = target.rsplit('/').next().unwrap_or(target.as_str());

    if target.starts_with("fixtures/") || target.contains("/fixtures/") {
        return RouteTargetRole::Example;
    }
    if matches!(
        target.as_str(),
        "readme.md" | "docs/readme.md" | "docs/index.md"
    ) {
        return RouteTargetRole::SharedHub;
    }
    if contains_any(&target, &["migration", "migrate", "upgrade"])
        || contains_any(&label, &["migration", "migrate", "upgrade"])
    {
        return RouteTargetRole::Migration;
    }
    if contains_any(&target, &["example", "examples", "sample", "samples"])
        || contains_any(&label, &["example", "sample"])
    {
        return RouteTargetRole::Example;
    }
    if canonical_target(route, &target, file_name)
        || (!target.contains('/') && canonical_label(route, &label))
    {
        return RouteTargetRole::Canonical;
    }
    if target.starts_with("docs/")
        || contains_any(&label, &["detail", "guide", "reference", "manual"])
    {
        return RouteTargetRole::Detail;
    }
    RouteTargetRole::Alternate
}

fn canonical_target(route: RouteKind, target: &str, file_name: &str) -> bool {
    match route {
        RouteKind::Identity => file_name == "readme.md",
        RouteKind::Docs => matches!(file_name, "docs.md" | "documentation.md"),
        RouteKind::Quickstart => matches!(
            file_name,
            "quickstart.md" | "quick-start.md" | "getting-started.md"
        ),
        RouteKind::Support => file_name == "support.md",
        RouteKind::Intake => target.starts_with(".github/issue_template/"),
        RouteKind::Contributing => file_name == "contributing.md",
        RouteKind::Security => file_name == "security.md",
        RouteKind::Release => {
            matches!(file_name, "changelog.md" | "releases.md")
                || (file_name == "release.md" && !target.contains('/'))
        }
        RouteKind::Lifecycle => matches!(file_name, "lifecycle.md" | "maintenance.md"),
        RouteKind::Governance => file_name == "governance.md",
        RouteKind::License => matches!(file_name, "license" | "license.md" | "copying"),
        RouteKind::Automation => target.starts_with(".github/workflows/"),
        RouteKind::Ownership => target == ".github/codeowners" || file_name == "codeowners",
        RouteKind::Hygiene => {
            matches!(file_name, ".gitignore" | ".gitattributes" | "hygiene.md")
        }
        RouteKind::Unknown => false,
    }
}

fn canonical_label(route: RouteKind, label: &str) -> bool {
    match route {
        RouteKind::Identity => contains_any(label, &["readme", "overview"]),
        RouteKind::Docs => contains_any(label, &["docs", "documentation"]),
        RouteKind::Quickstart => contains_any(label, &["quickstart", "getting started"]),
        RouteKind::Support => contains_any(label, &["support", "help"]),
        RouteKind::Intake => contains_any(label, &["issue", "bug", "feature request"]),
        RouteKind::Contributing => contains_any(label, &["contributing", "contribution"]),
        RouteKind::Security => contains_any(label, &["security", "vulnerability"]),
        RouteKind::Release => contains_any(label, &["release", "changelog"]),
        RouteKind::Lifecycle => contains_any(label, &["lifecycle", "maintenance"]),
        RouteKind::Governance => contains_any(label, &["governance", "decision"]),
        RouteKind::License => contains_any(label, &["license", "licence"]),
        RouteKind::Automation => contains_any(label, &["automation", "ci", "workflow"]),
        RouteKind::Ownership => contains_any(label, &["ownership", "codeowners", "maintainer"]),
        RouteKind::Hygiene => contains_any(label, &["hygiene", "gitignore", "gitattributes"]),
        RouteKind::Unknown => false,
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn is_fixture_document(path: &str) -> bool {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    path.starts_with("fixtures/") || path.contains("/fixtures/")
}
