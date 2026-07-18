use seiri_core::{
    DocumentEvent, DocumentProposition, DocumentPropositionKind, EvidenceAtom,
    MarkdownEvidenceKind, PropositionConflict, PropositionConflictSide, PropositionModality,
    RepositoryAnalysis, SourceSpan,
};

const MAX_DOCUMENT_PROPOSITIONS: usize = 512;
const MAX_PROPOSITION_CONFLICTS: usize = 512;
pub(crate) struct PropositionBuild {
    pub(crate) propositions: Vec<DocumentProposition>,
    pub(crate) conflicts: Vec<PropositionConflict>,
    pub(crate) truncated: bool,
}

pub(crate) fn build_proposition_consistency(snapshot: &RepositoryAnalysis) -> PropositionBuild {
    let mut propositions = Vec::new();
    let mut truncated = false;
    for entry in snapshot.document_index.scanned_documents() {
        if !entry.classification.is_primary_repository_content()
            || !is_current_claim_document(&entry.path)
        {
            continue;
        }
        let (Some(document_id), Some(scan)) = (entry.document_id, entry.scan.as_ref()) else {
            continue;
        };
        for event in scan.events() {
            let DocumentEvent::VisibleProse(prose) = event else {
                continue;
            };
            let Some(evidence) = evidence_for_markdown_event(
                snapshot,
                &entry.path,
                prose.span,
                MarkdownEvidenceKind::VisibleProse,
            ) else {
                continue;
            };
            for candidate in proposition_candidates(&prose.text) {
                if propositions.len() == MAX_DOCUMENT_PROPOSITIONS {
                    truncated = true;
                    break;
                }
                propositions.push(DocumentProposition {
                    id: String::new(),
                    kind: candidate.kind,
                    key: candidate.key,
                    value: candidate.value,
                    modality: candidate.modality,
                    document: document_id,
                    path: entry.path.clone(),
                    evidence,
                    span: prose.span,
                });
            }
        }
    }
    propositions.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.span.byte_start.cmp(&right.span.byte_start))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.value.cmp(&right.value))
    });
    propositions.dedup_by(|left, right| {
        left.path == right.path
            && left.span == right.span
            && left.kind == right.kind
            && left.key == right.key
            && left.value == right.value
            && left.modality == right.modality
    });
    for (index, proposition) in propositions.iter_mut().enumerate() {
        proposition.id = format!("document-proposition.{:04}", index + 1);
    }

    let mut conflicts = Vec::new();
    'pairs: for left_index in 0..propositions.len() {
        for right in propositions.iter().skip(left_index + 1) {
            let left = &propositions[left_index];
            if left.document == right.document
                || left.kind != right.kind
                || left.key != right.key
                || !propositions_compete(left, right)
            {
                continue;
            }
            if conflicts.len() == MAX_PROPOSITION_CONFLICTS {
                truncated = true;
                break 'pairs;
            }
            conflicts.push(PropositionConflict {
                id: format!("proposition-conflict.{:04}", conflicts.len() + 1),
                kind: left.kind,
                key: left.key.clone(),
                left: proposition_side(left),
                right: proposition_side(right),
                confidence_boundary: "candidate_only: deterministic visible-prose extraction can identify incompatible surface claims but does not establish author intent, policy validity, or factual correctness.".to_string(),
            });
        }
    }
    PropositionBuild {
        propositions,
        conflicts,
        truncated,
    }
}

fn is_current_claim_document(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
    !normalized.starts_with("docs/design/")
        && !normalized.contains("/archive/")
        && !normalized.contains("/history/")
        && !file_name.starts_with("changelog")
        && !file_name.contains("migration")
        && !file_name.contains("roadmap")
        && !file_name.contains("release-notes")
}

#[derive(Debug)]
struct PropositionCandidate {
    kind: DocumentPropositionKind,
    key: String,
    value: String,
    modality: PropositionModality,
}

fn proposition_candidates(text: &str) -> Vec<PropositionCandidate> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_lowercase();
    let mut output = Vec::new();
    if let Some(version) = semantic_version_token(&lower) {
        output.push(proposition(
            DocumentPropositionKind::Version,
            "project_version",
            version,
            PropositionModality::Affirmed,
        ));
    }
    for (kind, key, markers) in [
        (
            DocumentPropositionKind::Support,
            "support_window",
            &[
                "supported until ",
                "support ends ",
                "support through ",
                "サポート期限:",
                "サポート期限：",
            ][..],
        ),
        (
            DocumentPropositionKind::SecurityIntake,
            "security_intake",
            &[
                "report vulnerabilities to ",
                "security contact:",
                "security contact：",
                "脆弱性の報告先:",
                "脆弱性の報告先：",
            ][..],
        ),
        (
            DocumentPropositionKind::Release,
            "release_cadence",
            &[
                "release cadence:",
                "release cadence：",
                "release channel:",
                "release channel：",
                "リリース周期:",
                "リリース周期：",
                "リリースチャネル:",
                "リリースチャネル：",
            ][..],
        ),
    ] {
        if let Some(value) = value_after_any(&lower, markers) {
            output.push(proposition(kind, key, value, PropositionModality::Affirmed));
        }
    }
    for (marker, modality) in [
        ("does not support ", PropositionModality::Negated),
        ("doesn't support ", PropositionModality::Negated),
        ("supports ", PropositionModality::Affirmed),
    ] {
        if let Some(value) = value_after_any(&lower, &[marker]) {
            output.push(proposition(
                DocumentPropositionKind::Capability,
                "capability",
                value,
                modality,
            ));
            break;
        }
    }
    for (marker, modality) in [
        ("に対応しません", PropositionModality::Negated),
        ("に対応します", PropositionModality::Affirmed),
    ] {
        if let Some(index) = lower.find(marker) {
            let value = normalize_proposition_value(&lower[..index]);
            if !value.is_empty() {
                output.push(proposition(
                    DocumentPropositionKind::Capability,
                    "capability",
                    value,
                    modality,
                ));
                break;
            }
        }
    }
    output
}

fn proposition(
    kind: DocumentPropositionKind,
    key: &str,
    value: String,
    modality: PropositionModality,
) -> PropositionCandidate {
    PropositionCandidate {
        kind,
        key: key.to_string(),
        value,
        modality,
    }
}

fn semantic_version_token(value: &str) -> Option<String> {
    let has_marker = ["version ", "version:", "version：", "バージョン"]
        .iter()
        .any(|marker| value.contains(marker));
    has_marker.then_some(())?;
    value
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '+' | '_'))
        })
        .find_map(|token| {
            let normalized = token.trim_start_matches('v');
            (normalized.contains('.')
                && normalized
                    .chars()
                    .any(|character| character.is_ascii_digit())
                && normalized.len() <= 64)
                .then(|| normalized.to_string())
        })
}

fn value_after_any(value: &str, markers: &[&str]) -> Option<String> {
    markers.iter().find_map(|marker| {
        let start = value.find(marker)? + marker.len();
        let normalized = normalize_proposition_value(&value[start..]);
        (!normalized.is_empty()).then_some(normalized)
    })
}

fn normalize_proposition_value(value: &str) -> String {
    value
        .split(['.', '。', ';', '；', '\n'])
        .next()
        .unwrap_or_default()
        .trim_matches(|character: char| {
            character.is_whitespace()
                || matches!(character, '`' | '"' | '\'' | ':' | '：' | ',' | '、')
        })
        .split_whitespace()
        .take(16)
        .collect::<Vec<_>>()
        .join(" ")
}

fn propositions_compete(left: &DocumentProposition, right: &DocumentProposition) -> bool {
    match left.kind {
        DocumentPropositionKind::Capability => {
            left.value == right.value && left.modality != right.modality
        }
        _ => left.value != right.value || left.modality != right.modality,
    }
}

fn proposition_side(value: &DocumentProposition) -> PropositionConflictSide {
    PropositionConflictSide {
        proposition_id: value.id.clone(),
        path: value.path.clone(),
        evidence: value.evidence,
        span: value.span,
        value: value.value.clone(),
        modality: value.modality,
    }
}

fn evidence_for_markdown_event(
    snapshot: &RepositoryAnalysis,
    path: &str,
    span: SourceSpan,
    expected_event: MarkdownEvidenceKind,
) -> Option<seiri_core::EvidenceId> {
    snapshot.evidence_kernel.facts().iter().find_map(|fact| {
        let matches = snapshot.evidence_kernel.path_for_fact(fact) == Some(path)
            && matches!(
                fact.atom,
                EvidenceAtom::Markdown { event, .. } if event == expected_event
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
