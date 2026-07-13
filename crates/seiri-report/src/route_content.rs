use seiri_core::{
    route_content_contract, BilingualStructuralPair, ClaimBoundaryKind, ContentSlotAssessment,
    ContentSlotSpec, CoverageIndex, DocumentConsistencyReport, DocumentDiagnosticKind,
    DocumentEvent, DocumentIndex, EvidenceAtom, EvidenceId, EvidenceKernel, FacetReport,
    MarkdownEvidenceKind, MeaningAtomSet, Observation, PolicySensitivityWire, RouteContentReport,
    SourceSpan, UnknownReason,
};
use std::collections::BTreeSet;

pub(crate) fn build_route_content(
    kernel: &EvidenceKernel,
    coverage: &CoverageIndex,
    documents: &DocumentIndex,
    facets: &FacetReport,
    consistency: &DocumentConsistencyReport,
) -> RouteContentReport {
    let assessments = route_content_contract()
        .iter()
        .map(|spec| assess_slot(spec, kernel, coverage, documents, facets, consistency))
        .collect();
    RouteContentReport {
        assessments,
        structural_pairs: structural_pairs(documents, kernel),
        ..RouteContentReport::default()
    }
}

fn assess_slot(
    spec: &ContentSlotSpec,
    kernel: &EvidenceKernel,
    coverage: &CoverageIndex,
    documents: &DocumentIndex,
    facets: &FacetReport,
    consistency: &DocumentConsistencyReport,
) -> ContentSlotAssessment {
    let enabled = slot_enabled(spec, facets);
    let evidence = if enabled {
        matching_evidence(spec, kernel, documents)
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
    documents
        .scanned_documents()
        .filter(|entry| slot_allows_document(spec, entry))
        .find_map(|entry| {
            entry
                .scan
                .as_ref()?
                .diagnostics()
                .iter()
                .find_map(|diagnostic| match diagnostic.kind {
                    DocumentDiagnosticKind::UnsupportedHtml => {
                        Some(UnknownReason::UnsupportedSyntax)
                    }
                    DocumentDiagnosticKind::HtmlAttributeLimitExceeded => {
                        Some(UnknownReason::LimitExceeded)
                    }
                    DocumentDiagnosticKind::UnclosedLinkLabel
                    | DocumentDiagnosticKind::UnclosedLinkTarget
                    | DocumentDiagnosticKind::UnresolvedReferenceLink => None,
                })
        })
}

fn matching_evidence(
    spec: &ContentSlotSpec,
    kernel: &EvidenceKernel,
    documents: &DocumentIndex,
) -> Vec<EvidenceId> {
    let mut evidence = kernel
        .facts()
        .iter()
        .filter_map(|fact| match fact.atom {
            EvidenceAtom::ImportantFile(kind) if spec.important_files.contains(&kind) => {
                Some(fact.id)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    for entry in documents.scanned_documents() {
        if !slot_allows_document(spec, entry) {
            continue;
        }
        let Some(scan) = entry.scan.as_ref() else {
            continue;
        };
        for event in scan.events() {
            let Some((kind, span, searchable)) = searchable_event(event) else {
                continue;
            };
            if contains_any_normalized(&searchable, spec.markers) {
                evidence.extend(evidence_for_event(kernel, &entry.path, kind, span));
            }
        }
    }
    evidence.sort_unstable();
    evidence.dedup();
    evidence
}

fn slot_allows_document(spec: &ContentSlotSpec, entry: &seiri_core::IndexedDocument) -> bool {
    if !spec.document_roles.contains(entry.role) {
        return false;
    }
    match spec.scope {
        seiri_core::CoverageScope::RootReadme => entry.role == seiri_core::DocumentRole::RootReadme,
        seiri_core::CoverageScope::MarkdownDocuments => entry.scope_class.is_repository_content(),
        seiri_core::CoverageScope::DocumentRole(role) => entry.role == role,
        seiri_core::CoverageScope::Document(document) => entry.document_id == Some(document),
        seiri_core::CoverageScope::RepositoryFiles | seiri_core::CoverageScope::RemoteMetadata => {
            false
        }
    }
}

fn searchable_event(event: &DocumentEvent) -> Option<(MarkdownEvidenceKind, SourceSpan, String)> {
    match event {
        DocumentEvent::Heading(value) => Some((
            MarkdownEvidenceKind::Heading,
            value.span?,
            value.text.clone(),
        )),
        DocumentEvent::Link(value) => Some((
            MarkdownEvidenceKind::Link,
            value.span?,
            format!("{} {}", value.text, value.target),
        )),
        DocumentEvent::Badge(value) => Some((
            MarkdownEvidenceKind::Badge,
            value.span?,
            format!("{} {}", value.alt, value.target),
        )),
        DocumentEvent::RouteCandidate(value) => Some((
            MarkdownEvidenceKind::RouteCandidate,
            value.span?,
            value.target.as_ref().map_or_else(
                || value.text.clone(),
                |target| format!("{} {target}", value.text),
            ),
        )),
    }
}

fn contains_any_normalized(value: &str, markers: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    markers.iter().any(|marker| lower.contains(marker))
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
    kernel: &EvidenceKernel,
) -> Vec<BilingualStructuralPair> {
    let mut pairs = Vec::new();
    for entry in documents.scanned_documents() {
        if !entry.scope_class.is_repository_content() {
            continue;
        }
        let Some(scan) = entry.scan.as_ref() else {
            continue;
        };
        let sections = heading_sections(scan.events(), &entry.path, kernel);
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
    kernel: &EvidenceKernel,
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
            let mut evidence_ids =
                evidence_for_event(kernel, path, MarkdownEvidenceKind::Heading, span);
            for event in events {
                let DocumentEvent::Link(link) = event else {
                    continue;
                };
                let Some(link_span) = link.span else { continue };
                if link_span.byte_start > span.byte_start && link_span.byte_start < end {
                    targets.insert(normalize_target(&link.target));
                    evidence_ids.extend(evidence_for_event(
                        kernel,
                        path,
                        MarkdownEvidenceKind::Link,
                        link_span,
                    ));
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

fn evidence_for_event(
    kernel: &EvidenceKernel,
    path: &str,
    event: MarkdownEvidenceKind,
    span: SourceSpan,
) -> Vec<EvidenceId> {
    kernel
        .facts()
        .iter()
        .filter(|fact| {
            kernel.path_for_fact(fact) == Some(path)
                && matches!(fact.atom, EvidenceAtom::Markdown { event: actual, .. } if actual == event)
                && fact
                    .provenance
                    .span
                    .is_some_and(|actual| span_matches(actual, span))
        })
        .map(|fact| fact.id)
        .collect()
}

fn span_matches(actual: seiri_core::EvidenceSourceSpan, expected: SourceSpan) -> bool {
    actual.line.get() == u32::try_from(expected.line).unwrap_or(u32::MAX)
        && actual.column.get() == u32::try_from(expected.column).unwrap_or(u32::MAX)
        && actual.byte_start.get() == u32::try_from(expected.byte_start).unwrap_or(u32::MAX)
        && actual.byte_end.get() == u32::try_from(expected.byte_end).unwrap_or(u32::MAX)
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

#[must_use]
pub(crate) fn render_route_content_contract_markdown(report: &RouteContentReport) -> String {
    let mut out = String::from("## Route Content Contract\n\n");
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
