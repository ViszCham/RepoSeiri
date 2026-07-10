use seiri_core::{
    ClaimBoundaryKind, CodexLinterContext, WordingLintReport, WordingRuleKind,
    CODEX_KERNEL_SCHEMA_VERSION, CODEX_LINTER_CONTEXT_SCHEMA_VERSION,
};
use std::collections::BTreeSet;

pub(super) fn build_linter_context(report: Option<&WordingLintReport>) -> CodexLinterContext {
    let Some(report) = report else {
        return CodexLinterContext {
            schema_version: CODEX_LINTER_CONTEXT_SCHEMA_VERSION.to_string(),
            kernel_schema_version: CODEX_KERNEL_SCHEMA_VERSION.to_string(),
            source_schema_version: None,
            available: false,
            files_scanned: 0,
            generated_surfaces: 0,
            suppressed_boundary_exceptions: 0,
            findings: Vec::new(),
            rules: Vec::new(),
            boundary_kinds: Vec::new(),
            claim_boundary: linter_claim_boundary(),
        };
    };

    let rules = report
        .findings
        .iter()
        .map(|finding| finding.rule)
        .collect::<BTreeSet<WordingRuleKind>>()
        .into_iter()
        .collect();
    let boundary_kinds = report
        .findings
        .iter()
        .map(|finding| finding.boundary)
        .collect::<BTreeSet<ClaimBoundaryKind>>()
        .into_iter()
        .collect();
    CodexLinterContext {
        schema_version: CODEX_LINTER_CONTEXT_SCHEMA_VERSION.to_string(),
        kernel_schema_version: CODEX_KERNEL_SCHEMA_VERSION.to_string(),
        source_schema_version: Some(report.schema_version.clone()),
        available: true,
        files_scanned: report.summary.files_scanned,
        generated_surfaces: report.summary.generated_surfaces,
        suppressed_boundary_exceptions: report.summary.suppressed_boundary_exceptions,
        findings: report.findings.clone(),
        rules,
        boundary_kinds,
        claim_boundary: linter_claim_boundary(),
    }
}

fn linter_claim_boundary() -> String {
    "Codex linter context carries evidence-scoped wording findings from the shared review kernel. Findings are review hints, not legal, security, quality, trust, or publication-readiness judgments."
        .to_string()
}
