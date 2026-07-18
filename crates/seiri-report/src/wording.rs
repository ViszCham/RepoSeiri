use crate::{plan_to_markdown, AuditError};
use seiri_core::{
    stable_id, ClaimBoundaryKind, DocumentEvent, DocumentScan, RepositoryAnalysis,
    WordingBoundaryException, WordingLintFinding, WordingLintReport, WordingLintSourceKind,
    WordingLintSummary, WordingRuleKind, TOOL_NAME, WORDING_LINT_SCHEMA_VERSION,
};

const WORDING_BOUNDARY: &str = "Wording lint is a review aid for overclaim phrasing. It reports evidence-scoped wording risks only; it does not make legal, security, quality, popularity, trust, or publication-readiness judgments.";

#[derive(Debug, Clone, Copy)]
struct WordingRule {
    phrase: &'static str,
    rule: WordingRuleKind,
    boundary: ClaimBoundaryKind,
}

const WORDING_RULES: &[WordingRule] = &[
    WordingRule {
        phrase: "guarantees popularity",
        rule: WordingRuleKind::PopularityGuarantee,
        boundary: ClaimBoundaryKind::NotPopularityGuarantee,
    },
    WordingRule {
        phrase: "popularity guarantee",
        rule: WordingRuleKind::PopularityGuarantee,
        boundary: ClaimBoundaryKind::NotPopularityGuarantee,
    },
    WordingRule {
        phrase: "guarantees trust",
        rule: WordingRuleKind::TrustGuarantee,
        boundary: ClaimBoundaryKind::NotTrustGuarantee,
    },
    WordingRule {
        phrase: "trust guarantee",
        rule: WordingRuleKind::TrustGuarantee,
        boundary: ClaimBoundaryKind::NotTrustGuarantee,
    },
    WordingRule {
        phrase: "guarantees security",
        rule: WordingRuleKind::SecurityGuarantee,
        boundary: ClaimBoundaryKind::NotSecurityGuarantee,
    },
    WordingRule {
        phrase: "security guarantee",
        rule: WordingRuleKind::SecurityGuarantee,
        boundary: ClaimBoundaryKind::NotSecurityGuarantee,
    },
    WordingRule {
        phrase: "guarantees quality",
        rule: WordingRuleKind::QualityGuarantee,
        boundary: ClaimBoundaryKind::NotQualityGuarantee,
    },
    WordingRule {
        phrase: "quality guarantee",
        rule: WordingRuleKind::QualityGuarantee,
        boundary: ClaimBoundaryKind::NotQualityGuarantee,
    },
    WordingRule {
        phrase: "legally compliant",
        rule: WordingRuleKind::LegalFitnessGuarantee,
        boundary: ClaimBoundaryKind::NotLegalFitnessGuarantee,
    },
    WordingRule {
        phrase: "legal fitness",
        rule: WordingRuleKind::LegalFitnessGuarantee,
        boundary: ClaimBoundaryKind::NotLegalFitnessGuarantee,
    },
    WordingRule {
        phrase: "legal advice",
        rule: WordingRuleKind::LegalAdvice,
        boundary: ClaimBoundaryKind::NotLegalAdvice,
    },
    WordingRule {
        phrase: "guarantees maintenance",
        rule: WordingRuleKind::MaintenanceGuarantee,
        boundary: ClaimBoundaryKind::NotMaintenanceGuarantee,
    },
    WordingRule {
        phrase: "maintenance guarantee",
        rule: WordingRuleKind::MaintenanceGuarantee,
        boundary: ClaimBoundaryKind::NotMaintenanceGuarantee,
    },
    WordingRule {
        phrase: "runtime verified",
        rule: WordingRuleKind::RuntimeVerification,
        boundary: ClaimBoundaryKind::NotRuntimeVerification,
    },
    WordingRule {
        phrase: "runtime verification",
        rule: WordingRuleKind::RuntimeVerification,
        boundary: ClaimBoundaryKind::NotRuntimeVerification,
    },
    WordingRule {
        phrase: "publication ready",
        rule: WordingRuleKind::PublicationReadiness,
        boundary: ClaimBoundaryKind::NotPublicationReadiness,
    },
    WordingRule {
        phrase: "production ready",
        rule: WordingRuleKind::ProductionReadiness,
        boundary: ClaimBoundaryKind::NotProductionReadiness,
    },
    WordingRule {
        phrase: "production-ready",
        rule: WordingRuleKind::ProductionReadiness,
        boundary: ClaimBoundaryKind::NotProductionReadiness,
    },
    WordingRule {
        phrase: "automatically adopts policies",
        rule: WordingRuleKind::AutomaticPolicyAdoption,
        boundary: ClaimBoundaryKind::NotAutomaticPolicyAdoption,
    },
    WordingRule {
        phrase: "automatically adopt policies",
        rule: WordingRuleKind::AutomaticPolicyAdoption,
        boundary: ClaimBoundaryKind::NotAutomaticPolicyAdoption,
    },
    WordingRule {
        phrase: "automatically adopts weights",
        rule: WordingRuleKind::AutomaticWeightAdoption,
        boundary: ClaimBoundaryKind::NotAutomaticWeightAdoption,
    },
    WordingRule {
        phrase: "automatically adopt weights",
        rule: WordingRuleKind::AutomaticWeightAdoption,
        boundary: ClaimBoundaryKind::NotAutomaticWeightAdoption,
    },
    WordingRule {
        phrase: "guaranteed",
        rule: WordingRuleKind::GenericGuarantee,
        boundary: ClaimBoundaryKind::NotQualityGuarantee,
    },
    WordingRule {
        phrase: "guarantees",
        rule: WordingRuleKind::GenericGuarantee,
        boundary: ClaimBoundaryKind::NotQualityGuarantee,
    },
    WordingRule {
        phrase: "guarantee",
        rule: WordingRuleKind::GenericGuarantee,
        boundary: ClaimBoundaryKind::NotQualityGuarantee,
    },
];

#[derive(Debug)]
struct WordingTextTarget {
    source: WordingLintSourceKind,
    path: String,
    text: String,
    base_line: usize,
    base_column: usize,
    base_byte: usize,
}

#[derive(Debug, Clone, Copy)]
struct RawMatch {
    start: usize,
    end: usize,
    rule: WordingRule,
    rule_index: usize,
}

pub(crate) fn lint_source_session(
    analysis: &RepositoryAnalysis,
    source_documents: &[seiri_markdown::SourceDocument],
) -> Result<WordingLintReport, AuditError> {
    let repo_root = ".".to_string();
    let mut targets = Vec::new();
    let mut repository_files = 0usize;

    for document in source_documents {
        if !is_wording_source_path(document.path()) {
            continue;
        }
        repository_files += 1;
        let Some(scan) = analysis
            .document_index
            .entries()
            .iter()
            .find(|entry| entry.path == document.path())
            .and_then(|entry| entry.scan.as_ref())
        else {
            continue;
        };
        targets.extend(visible_prose_targets(
            WordingLintSourceKind::RepositoryFile,
            document.path(),
            document.text(),
            scan,
        ));
    }

    let plan = seiri_planner::plan_patches(analysis);
    let generated = generated_report_targets(analysis, &plan)?;
    let generated_surfaces = 3;
    targets.extend(generated);

    let mut findings = Vec::new();
    let mut suppressed_boundary_exceptions = 0;
    for target in &targets {
        lint_text_target(target, &mut findings, &mut suppressed_boundary_exceptions);
    }
    for (index, finding) in findings.iter_mut().enumerate() {
        finding.id = stable_id("wording", index + 1);
    }

    Ok(WordingLintReport {
        schema_version: WORDING_LINT_SCHEMA_VERSION.to_string(),
        tool: TOOL_NAME.to_string(),
        repo_root,
        summary: WordingLintSummary {
            files_scanned: repository_files,
            generated_surfaces,
            findings: findings.len(),
            suppressed_boundary_exceptions,
        },
        findings,
        boundary: WORDING_BOUNDARY.to_string(),
    })
}

pub(crate) fn render_markdown(report: &WordingLintReport) -> String {
    let mut out = String::new();
    out.push_str("# RepoSeiri Wording Lint Report\n\n");
    out.push_str(&format!("- Schema: `{}`\n", report.schema_version));
    out.push_str(&format!("- Repository: `{}`\n", report.repo_root));
    out.push_str(&format!(
        "- Files scanned: `{}`\n",
        report.summary.files_scanned
    ));
    out.push_str(&format!(
        "- Generated surfaces: `{}`\n",
        report.summary.generated_surfaces
    ));
    out.push_str(&format!("- Findings: `{}`\n", report.summary.findings));
    out.push_str(&format!(
        "- Suppressed boundary exceptions: `{}`\n",
        report.summary.suppressed_boundary_exceptions
    ));
    out.push_str(&format!("- Boundary: {}\n\n", report.boundary));

    out.push_str("## Findings\n\n");
    if report.findings.is_empty() {
        out.push_str("- No overclaim wording findings emitted.\n");
        return out;
    }

    for finding in &report.findings {
        out.push_str(&format!("### {}\n\n", finding.id));
        out.push_str(&format!("- Source: `{:?}`\n", finding.source));
        out.push_str(&format!("- Path: `{}`\n", finding.path));
        out.push_str(&format!("- Line: `{}`\n", finding.line));
        out.push_str(&format!("- Column: `{}`\n", finding.column));
        out.push_str(&format!(
            "- Byte range: `{}`..`{}`\n",
            finding.byte_start, finding.byte_end
        ));
        out.push_str(&format!("- Matched: `{}`\n", finding.matched));
        out.push_str(&format!("- Rule: `{:?}`\n", finding.rule));
        out.push_str(&format!("- Boundary: `{:?}`\n", finding.boundary));
        out.push_str(&format!(
            "- Replacement hint: {}\n\n",
            finding.replacement_hint
        ));
    }
    out
}

fn generated_report_targets(
    snapshot: &RepositoryAnalysis,
    plan: &seiri_core::PatchPlan,
) -> Result<Vec<WordingTextTarget>, AuditError> {
    let view = seiri_codex::CodexView::new(snapshot, plan, None);
    let generated = [
        ("generated/audit.md", crate::to_markdown(snapshot)),
        ("generated/plan.md", plan_to_markdown(plan)),
        (
            "generated/codex.md",
            seiri_codex::render_query_markdown(
                &view.query(seiri_codex::CodexQueryKind::Governance),
            ),
        ),
    ];
    let mut targets = Vec::new();
    for (path, text) in generated {
        let scan = seiri_markdown::scan_document_with_options(
            path,
            &text,
            &seiri_markdown::DocumentScanOptions::derived_for_source(text.len()),
        )?;
        targets.extend(visible_prose_targets(
            WordingLintSourceKind::GeneratedReport,
            path,
            &text,
            &scan,
        ));
    }
    Ok(targets)
}

fn lint_text_target(
    target: &WordingTextTarget,
    findings: &mut Vec<WordingLintFinding>,
    suppressed_boundary_exceptions: &mut usize,
) {
    let mut matches = raw_matches(&target.text);
    matches.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
            .then_with(|| left.rule_index.cmp(&right.rule_index))
    });

    let mut accepted_spans = Vec::<(usize, usize)>::new();
    for raw in matches {
        if accepted_spans
            .iter()
            .any(|(start, end)| spans_overlap(raw.start, raw.end, *start, *end))
        {
            continue;
        }
        if boundary_exception(&target.text, raw.start, raw.end).is_some() {
            *suppressed_boundary_exceptions += 1;
            continue;
        }
        accepted_spans.push((raw.start, raw.end));
        let (relative_line, relative_column) = line_column(&target.text, raw.start);
        let line = target.base_line + relative_line - 1;
        let column = if relative_line == 1 {
            target.base_column + relative_column - 1
        } else {
            relative_column
        };
        findings.push(WordingLintFinding {
            id: String::new(),
            source: target.source,
            path: target.path.clone(),
            line,
            column,
            byte_start: target.base_byte + raw.start,
            byte_end: target.base_byte + raw.end,
            matched: target.text[raw.start..raw.end].to_string(),
            rule: raw.rule.rule,
            boundary: raw.rule.boundary,
            replacement_hint: replacement_hint(raw.rule.boundary).to_string(),
        });
    }
}

fn visible_prose_targets(
    source: WordingLintSourceKind,
    path: &str,
    original: &str,
    scan: &DocumentScan,
) -> Vec<WordingTextTarget> {
    scan.events()
        .iter()
        .filter_map(|event| {
            let DocumentEvent::VisibleProse(prose) = event else {
                return None;
            };
            let text = original
                .get(prose.span.byte_start..prose.span.byte_end)?
                .to_string();
            (!text.trim().is_empty()).then(|| WordingTextTarget {
                source,
                path: path.to_string(),
                text,
                base_line: prose.span.line,
                base_column: prose.span.column,
                base_byte: prose.span.byte_start,
            })
        })
        .collect()
}

fn raw_matches(text: &str) -> Vec<RawMatch> {
    let mut matches = Vec::new();
    for (rule_index, rule) in WORDING_RULES.iter().copied().enumerate() {
        for start in find_ascii_case_insensitive(text, rule.phrase) {
            matches.push(RawMatch {
                start,
                end: start + rule.phrase.len(),
                rule,
                rule_index,
            });
        }
    }
    matches
}

fn find_ascii_case_insensitive(text: &str, phrase: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let needle = phrase.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return Vec::new();
    }

    let mut starts = Vec::new();
    for start in 0..=(bytes.len() - needle.len()) {
        let end = start + needle.len();
        if !is_word_boundary(bytes, start, end) {
            continue;
        }
        if bytes[start..end]
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
        {
            starts.push(start);
        }
    }
    starts
}

fn is_word_boundary(bytes: &[u8], start: usize, end: usize) -> bool {
    let before = start.checked_sub(1).and_then(|index| bytes.get(index));
    let after = bytes.get(end);
    !before.is_some_and(|byte| is_word_byte(*byte))
        && !after.is_some_and(|byte| is_word_byte(*byte))
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn boundary_exception(text: &str, start: usize, end: usize) -> Option<WordingBoundaryException> {
    if typed_boundary_token(text, start, end) {
        return Some(WordingBoundaryException::TypedClaimBoundary);
    }
    if typed_boundary_vocabulary_context(text, start, end) {
        return Some(WordingBoundaryException::TypedClaimBoundary);
    }
    if negated_boundary_context(text, start) {
        return Some(WordingBoundaryException::NegatedBoundaryStatement);
    }
    None
}

fn typed_boundary_vocabulary_context(text: &str, start: usize, end: usize) -> bool {
    let line_start = text[..start]
        .rfind('\n')
        .map_or(0, |index| index.saturating_add(1));
    let line_end = text[end..]
        .find('\n')
        .map_or(text.len(), |offset| end + offset);
    let before = text[line_start..start].to_ascii_lowercase();
    let after = text[end..line_end].to_ascii_lowercase();
    before.contains("blocked ") && (after.contains(" boundary") || after.contains(" boundaries"))
}

fn typed_boundary_token(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    let token_start = scan_token_start(bytes, start);
    let token_end = scan_token_end(bytes, end);
    let token = text[token_start..token_end]
        .replace(['-', '_', ':'], "")
        .to_ascii_lowercase();
    token.starts_with("not") && token.contains("guarantee")
}

fn scan_token_start(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    while index > 0 {
        let previous = bytes[index - 1];
        if !(previous.is_ascii_alphanumeric()
            || previous == b'_'
            || previous == b'-'
            || previous == b':')
        {
            break;
        }
        index -= 1;
    }
    index
}

fn scan_token_end(bytes: &[u8], end: usize) -> usize {
    let mut index = end;
    while index < bytes.len() {
        let current = bytes[index];
        if !(current.is_ascii_alphanumeric()
            || current == b'_'
            || current == b'-'
            || current == b':')
        {
            break;
        }
        index += 1;
    }
    index
}

fn negated_boundary_context(text: &str, start: usize) -> bool {
    let line_start = text[..start]
        .rfind('\n')
        .map_or(0, |index| index.saturating_add(1));
    let context = text[line_start..start].to_ascii_lowercase();
    [
        "does not ",
        "do not ",
        "not ",
        "never ",
        "no ",
        "without ",
        "cannot ",
        "can't ",
        "must not ",
    ]
    .iter()
    .any(|marker| context.contains(marker))
}

fn line_column(text: &str, byte_start: usize) -> (usize, usize) {
    let mut line = 1;
    let mut line_start = 0;
    for (index, character) in text.char_indices() {
        if index >= byte_start {
            break;
        }
        if character == '\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    let column = text[line_start..byte_start].chars().count() + 1;
    (line, column)
}

fn spans_overlap(left_start: usize, left_end: usize, right_start: usize, right_end: usize) -> bool {
    left_start < right_end && right_start < left_end
}

fn replacement_hint(boundary: ClaimBoundaryKind) -> &'static str {
    match boundary {
        ClaimBoundaryKind::NotPopularityGuarantee => {
            "Use evidence-scoped popularity wording such as `observed in the scanned sources`."
        }
        ClaimBoundaryKind::NotTrustGuarantee => {
            "Use trust-route wording such as `trust-related files are present` instead of trust guarantees."
        }
        ClaimBoundaryKind::NotSecurityGuarantee => {
            "Use security-route wording such as `security documentation or automation is present` instead of security guarantees."
        }
        ClaimBoundaryKind::NotQualityGuarantee => {
            "Use evidence-scoped wording such as `the audit observed` or `evidence suggests` instead of guarantee language."
        }
        ClaimBoundaryKind::NotLegalFitnessGuarantee | ClaimBoundaryKind::NotLegalAdvice => {
            "Use review-boundary wording and keep legal decisions with maintainers."
        }
        ClaimBoundaryKind::NotMaintenanceGuarantee => {
            "Describe observed maintenance routes without promising future maintenance."
        }
        ClaimBoundaryKind::NotRuntimeVerification => {
            "Describe static review evidence without claiming runtime verification."
        }
        ClaimBoundaryKind::NotPublicationReadiness => {
            "Describe publication-readiness signals without saying publication is ready."
        }
        ClaimBoundaryKind::NotProductionReadiness => {
            "Describe production-readiness signals without claiming production readiness."
        }
        ClaimBoundaryKind::NotAutomaticPolicyAdoption => {
            "Say policy adoption requires maintainer review."
        }
        ClaimBoundaryKind::NotAutomaticWeightAdoption => {
            "Say weight changes are candidate evidence and require maintainer review."
        }
        ClaimBoundaryKind::NotOwnerApproval => "Say owner approval requires explicit maintainer action.",
    }
}

fn is_wording_source_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or(&lower);
    let is_readme = !lower.contains('/') && basename.starts_with("readme");
    let is_docs = lower.starts_with("docs/");
    (is_readme || is_docs) && is_text_doc_path(&lower)
}

fn is_text_doc_path(path: &str) -> bool {
    path.ends_with(".md")
        || path.ends_with(".mdx")
        || path.ends_with(".txt")
        || path.ends_with(".rst")
        || !path.rsplit('/').next().unwrap_or(path).contains('.')
}
