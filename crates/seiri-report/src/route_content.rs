use seiri_core::{
    route_content_contract_v2, BilingualStructuralPair, ClaimBoundaryKind, ContentObservation,
    ContentSlotAssessment, ContentSlotSpec, CoverageIndex, DocumentConsistencyReport,
    DocumentDiagnosticKind, DocumentEvent, DocumentIndex, EvidenceFact, EvidenceId, EvidenceKernel,
    EvidenceKind, FacetReport, ImportantFileKind, MeaningAtomSet, Observation,
    PolicySensitivityWire, RouteContentAssessment, RouteContentAtom, RouteContentAtomAssessment,
    RouteContentReportV2, RouteKind, SourceSpan, UnknownReason,
};
use std::collections::BTreeSet;

pub(crate) fn build_route_content(
    kernel: &EvidenceKernel,
    coverage: &CoverageIndex,
) -> Vec<RouteContentAssessment> {
    route_content_routes()
        .iter()
        .map(|route| RouteContentAssessment {
            route: *route,
            atoms: RouteContentAtom::ALL
                .into_iter()
                .filter(|atom| atom.route() == *route)
                .map(|atom| RouteContentAtomAssessment {
                    atom,
                    observation: observe_legacy_atom(atom, kernel.facts(), coverage),
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn build_route_content_v2(
    kernel: &EvidenceKernel,
    coverage: &CoverageIndex,
    documents: &DocumentIndex,
    facets: &FacetReport,
    consistency: &DocumentConsistencyReport,
) -> RouteContentReportV2 {
    let assessments = route_content_contract_v2()
        .iter()
        .map(|spec| {
            assess_slot(
                spec,
                kernel.facts(),
                coverage,
                documents,
                facets,
                consistency,
            )
        })
        .collect();
    RouteContentReportV2 {
        assessments,
        structural_pairs: structural_pairs(documents, kernel.facts()),
        ..RouteContentReportV2::default()
    }
}

fn assess_slot(
    spec: &ContentSlotSpec,
    facts: &[EvidenceFact],
    coverage: &CoverageIndex,
    documents: &DocumentIndex,
    facets: &FacetReport,
    consistency: &DocumentConsistencyReport,
) -> ContentSlotAssessment {
    let enabled = slot_enabled(spec, facets);
    let evidence = if enabled {
        matching_evidence(spec, facts)
    } else {
        Vec::new()
    };
    let conflict_evidence = consistency
        .conflicts
        .iter()
        .filter(|conflict| conflict.route == spec.route)
        .flat_map(|conflict| [conflict.left.evidence, conflict.right.evidence])
        .collect::<Vec<_>>();
    let observation = if !enabled {
        Observation::Unknown(UnknownReason::NotRequested)
    } else if !conflict_evidence.is_empty() {
        Observation::conflict(conflict_evidence).expect("document conflicts retain evidence")
    } else if !evidence.is_empty() {
        Observation::present(MeaningAtomSet(spec.indicates.to_vec()), evidence)
            .expect("matched content slots retain evidence")
    } else if let Some(reason) = diagnostic_unknown_reason(spec, documents) {
        Observation::Unknown(reason)
    } else {
        coverage.observe_absence(spec.scope)
    };
    ContentSlotAssessment {
        slot: spec.id,
        code: spec.code.to_string(),
        route: spec.route,
        enabled,
        condition_evidence_ids: facet_condition_evidence(spec, facets),
        sensitivity: PolicySensitivityWire::from(spec.sensitivity),
        observation,
        indicates: spec.indicates.to_vec(),
        does_not_indicate: spec.does_not_indicate.to_vec(),
    }
}

fn facet_condition_evidence(spec: &ContentSlotSpec, facets: &FacetReport) -> Vec<EvidenceId> {
    let mut ids = spec
        .enabled_by_any_facet
        .iter()
        .filter_map(|facet| facets.observed_evidence(*facet))
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn slot_enabled(spec: &ContentSlotSpec, facets: &FacetReport) -> bool {
    spec.enabled_by_any_facet.is_empty()
        || spec.enabled_by_any_facet.iter().any(|facet| {
            facets.assessment(*facet).is_some_and(|assessment| {
                matches!(
                    assessment.observation,
                    Observation::Present { .. } | Observation::Conflict { .. }
                )
            })
        })
}

fn diagnostic_unknown_reason(
    spec: &ContentSlotSpec,
    documents: &DocumentIndex,
) -> Option<UnknownReason> {
    if !matches!(
        spec.scope,
        seiri_core::CoverageScope::MarkdownDocuments
            | seiri_core::CoverageScope::RootReadme
            | seiri_core::CoverageScope::DocumentRole(_)
            | seiri_core::CoverageScope::Document(_)
    ) {
        return None;
    }
    documents.scanned_documents().find_map(|entry| {
        entry
            .scan
            .as_ref()?
            .diagnostics()
            .iter()
            .find_map(|diagnostic| match diagnostic.kind {
                DocumentDiagnosticKind::UnsupportedHtml => Some(UnknownReason::UnsupportedSyntax),
                DocumentDiagnosticKind::HtmlAttributeLimitExceeded => {
                    Some(UnknownReason::LimitExceeded)
                }
                DocumentDiagnosticKind::UnclosedLinkLabel
                | DocumentDiagnosticKind::UnclosedLinkTarget
                | DocumentDiagnosticKind::UnresolvedReferenceLink => None,
            })
    })
}

fn matching_evidence(spec: &ContentSlotSpec, facts: &[EvidenceFact]) -> Vec<EvidenceId> {
    let mut evidence = facts
        .iter()
        .filter(|fact| fact_matches_spec(fact, spec))
        .map(|fact| fact.id)
        .collect::<Vec<_>>();
    evidence.sort_unstable();
    evidence.dedup();
    evidence
}

fn fact_matches_spec(fact: &EvidenceFact, spec: &ContentSlotSpec) -> bool {
    if fact.kind == EvidenceKind::ImportantFile {
        return spec
            .important_files
            .iter()
            .any(|kind| important_file_value(*kind) == fact.value);
    }
    is_markdown_content_fact(fact.kind) && contains_any_normalized(&fact.value, spec.markers)
}

fn observe_legacy_atom(
    atom: RouteContentAtom,
    facts: &[EvidenceFact],
    coverage: &CoverageIndex,
) -> ContentObservation {
    let mut evidence = route_content_contract_v2()
        .iter()
        .filter(|spec| spec.legacy_atom == Some(atom))
        .flat_map(|spec| matching_legacy_evidence(spec, facts))
        .collect::<Vec<_>>();
    if atom == RouteContentAtom::AutomationStatusSignal {
        evidence.extend(
            facts
                .iter()
                .filter(|fact| fact.kind == EvidenceKind::MarkdownBadge)
                .map(|fact| fact.id),
        );
    }
    evidence.sort_unstable();
    evidence.dedup();
    if evidence.is_empty() {
        return ContentObservation::from(
            coverage.observe_absence::<()>(seiri_core::CoverageScope::MarkdownDocuments),
        );
    }
    ContentObservation::from(
        Observation::present((), evidence)
            .expect("matched route content atoms always retain evidence identifiers"),
    )
}

fn matching_legacy_evidence(spec: &ContentSlotSpec, facts: &[EvidenceFact]) -> Vec<EvidenceId> {
    facts
        .iter()
        .filter(|fact| {
            if fact.kind == EvidenceKind::ImportantFile {
                return spec
                    .legacy_important_files
                    .iter()
                    .any(|kind| important_file_value(*kind) == fact.value);
            }
            is_markdown_content_fact(fact.kind)
                && contains_any_normalized(&fact.value, spec.legacy_markers)
        })
        .map(|fact| fact.id)
        .collect()
}

fn is_markdown_content_fact(kind: EvidenceKind) -> bool {
    matches!(
        kind,
        EvidenceKind::MarkdownHeading | EvidenceKind::MarkdownLink | EvidenceKind::RouteCandidate
    )
}

fn contains_any_normalized(value: &str, markers: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    markers.iter().any(|marker| lower.contains(marker))
}

fn important_file_value(kind: ImportantFileKind) -> &'static str {
    match kind {
        ImportantFileKind::Readme => "Readme",
        ImportantFileKind::License => "License",
        ImportantFileKind::Contributing => "Contributing",
        ImportantFileKind::Security => "Security",
        ImportantFileKind::Support => "Support",
        ImportantFileKind::IssueTemplate => "IssueTemplate",
        ImportantFileKind::IssueForm => "IssueForm",
        ImportantFileKind::PullRequestTemplate => "PullRequestTemplate",
        ImportantFileKind::Changelog => "Changelog",
        ImportantFileKind::Codeowners => "Codeowners",
        ImportantFileKind::CargoToml => "CargoToml",
        ImportantFileKind::DocsDirectory => "DocsDirectory",
        ImportantFileKind::Workflow => "Workflow",
        ImportantFileKind::DependencyBot => "DependencyBot",
        ImportantFileKind::SecurityAutomation => "SecurityAutomation",
        ImportantFileKind::Gitignore => "Gitignore",
        ImportantFileKind::Gitattributes => "Gitattributes",
        ImportantFileKind::EditorConfig => "EditorConfig",
    }
}

#[derive(Debug)]
struct HeadingSection {
    span: SourceSpan,
    level: u8,
    language: HeadingLanguage,
    targets: BTreeSet<String>,
    evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadingLanguage {
    Japanese,
    English,
    MixedOrUnknown,
}

fn structural_pairs(
    documents: &DocumentIndex,
    facts: &[EvidenceFact],
) -> Vec<BilingualStructuralPair> {
    let mut pairs = Vec::new();
    for entry in documents.scanned_documents() {
        let Some(scan) = entry.scan.as_ref() else {
            continue;
        };
        let sections = heading_sections(scan.events(), &entry.path, facts);
        for adjacent in sections.windows(2) {
            let [left, right] = adjacent else { continue };
            if left.level != right.level
                || !matches!(
                    (left.language, right.language),
                    (HeadingLanguage::Japanese, HeadingLanguage::English)
                        | (HeadingLanguage::English, HeadingLanguage::Japanese)
                )
                || left.targets.is_empty()
                || left.targets != right.targets
            {
                continue;
            }
            let mut evidence_ids = left
                .evidence_ids
                .iter()
                .chain(&right.evidence_ids)
                .copied()
                .collect::<Vec<_>>();
            evidence_ids.sort_unstable();
            evidence_ids.dedup();
            pairs.push(BilingualStructuralPair {
                document_path: entry.path.clone(),
                left_heading: left.span,
                right_heading: right.span,
                normalized_targets: left.targets.iter().cloned().collect(),
                evidence_ids,
                candidate_only: true,
            });
        }
    }
    pairs.sort_by(|left, right| {
        (&left.document_path, left.left_heading.byte_start)
            .cmp(&(&right.document_path, right.left_heading.byte_start))
    });
    pairs
}

fn heading_sections(
    events: &[DocumentEvent],
    path: &str,
    facts: &[EvidenceFact],
) -> Vec<HeadingSection> {
    let headings = events
        .iter()
        .filter_map(|event| match event {
            DocumentEvent::Heading(heading) => Some(heading),
            _ => None,
        })
        .collect::<Vec<_>>();
    headings
        .iter()
        .enumerate()
        .filter_map(|(index, heading)| {
            let span = heading.span?;
            let end = headings
                .iter()
                .skip(index + 1)
                .find(|next| next.level <= heading.level)
                .and_then(|next| next.span)
                .map_or(usize::MAX, |next| next.byte_start);
            let mut targets = BTreeSet::new();
            let mut evidence_ids = evidence_for_span(path, span, facts);
            for event in events {
                let DocumentEvent::Link(link) = event else {
                    continue;
                };
                let Some(link_span) = link.span else { continue };
                if link_span.byte_start > span.byte_start && link_span.byte_start < end {
                    targets.insert(normalize_target(&link.target));
                    evidence_ids.extend(evidence_for_span(path, link_span, facts));
                }
            }
            evidence_ids.sort_unstable();
            evidence_ids.dedup();
            Some(HeadingSection {
                span,
                level: heading.level,
                language: heading_language(&heading.text),
                targets,
                evidence_ids,
            })
        })
        .collect()
}

fn evidence_for_span(path: &str, span: SourceSpan, facts: &[EvidenceFact]) -> Vec<EvidenceId> {
    facts
        .iter()
        .filter(|fact| fact.path.as_deref() == Some(path) && fact.span == Some(span))
        .map(|fact| fact.id)
        .collect()
}

fn normalize_target(target: &str) -> String {
    let mut normalized = target.trim().replace('\\', "/");
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_string();
    }
    normalized.make_ascii_lowercase();
    normalized
}

fn heading_language(text: &str) -> HeadingLanguage {
    let japanese = text
        .chars()
        .any(|ch| matches!(ch as u32, 0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff));
    let english = text.chars().any(|ch| ch.is_ascii_alphabetic());
    match (japanese, english) {
        (true, false) => HeadingLanguage::Japanese,
        (false, true) => HeadingLanguage::English,
        _ => HeadingLanguage::MixedOrUnknown,
    }
}

fn route_content_routes() -> &'static [RouteKind] {
    &[
        RouteKind::Identity,
        RouteKind::Docs,
        RouteKind::Quickstart,
        RouteKind::Support,
        RouteKind::Intake,
        RouteKind::Contributing,
        RouteKind::Security,
        RouteKind::Release,
        RouteKind::Lifecycle,
        RouteKind::Governance,
        RouteKind::License,
        RouteKind::Automation,
        RouteKind::Ownership,
        RouteKind::Hygiene,
    ]
}

#[must_use]
pub(crate) fn render_route_content_contract_markdown(report: &RouteContentReportV2) -> String {
    let mut out = String::from("## Route Content Contract v2\n\n");
    for assessment in &report.assessments {
        let state = match &assessment.observation {
            Observation::Present { .. } => "present",
            Observation::Absent { .. } => "absent",
            Observation::Unknown(_) => "unknown",
            Observation::Conflict { .. } => "conflict",
        };
        let meanings = assessment
            .indicates
            .iter()
            .map(|value| format!("`{value:?}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let boundaries = assessment
            .does_not_indicate
            .iter()
            .map(boundary_label)
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "- `{}`: **{}**; indicates {}",
            assessment.code, state, meanings
        ));
        if !boundaries.is_empty() {
            out.push_str(&format!("; does not indicate {boundaries}"));
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "\n- Structural JA/EN candidates: `{}`\n- Boundary: {}\n",
        report.structural_pairs.len(),
        report.boundary
    ));
    out
}

fn boundary_label(boundary: &ClaimBoundaryKind) -> &'static str {
    match boundary {
        ClaimBoundaryKind::NotSecurityGuarantee => "`NotSecurityGuarantee`",
        ClaimBoundaryKind::NotRuntimeVerification => "`NotRuntimeVerification`",
        ClaimBoundaryKind::NotLegalFitnessGuarantee => "`NotLegalFitnessGuarantee`",
        ClaimBoundaryKind::NotLegalAdvice => "`NotLegalAdvice`",
        ClaimBoundaryKind::NotMaintenanceGuarantee => "`NotMaintenanceGuarantee`",
        ClaimBoundaryKind::NotOwnerApproval => "`NotOwnerApproval`",
        ClaimBoundaryKind::NotPopularityGuarantee => "`NotPopularityGuarantee`",
        ClaimBoundaryKind::NotTrustGuarantee => "`NotTrustGuarantee`",
        ClaimBoundaryKind::NotQualityGuarantee => "`NotQualityGuarantee`",
        ClaimBoundaryKind::NotPublicationReadiness => "`NotPublicationReadiness`",
        ClaimBoundaryKind::NotProductionReadiness => "`NotProductionReadiness`",
        ClaimBoundaryKind::NotAutomaticPolicyAdoption => "`NotAutomaticPolicyAdoption`",
        ClaimBoundaryKind::NotAutomaticWeightAdoption => "`NotAutomaticWeightAdoption`",
    }
}
