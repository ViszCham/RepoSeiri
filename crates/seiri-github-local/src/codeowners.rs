use crate::{
    check_diagnostics, malformed_failure, scalar_within_limit, ParseFailure, StructuredParseOptions,
};
use seiri_core::{CodeownerEntry, Codeowners, GithubDiagnostic, GithubDiagnosticKind, SourceSpan};

pub(crate) fn parse_codeowners(
    source: &str,
    options: &StructuredParseOptions,
) -> Result<(Codeowners, Vec<GithubDiagnostic>), ParseFailure> {
    let mut entries = Vec::new();
    let diagnostics = Vec::new();
    let mut byte_start = 0usize;
    for (line_index, segment) in source.split_inclusive('\n').enumerate() {
        let raw = strip_line_ending(segment);
        let span = SourceSpan::new(line_index + 1, 1, byte_start, byte_start + raw.len());
        byte_start = byte_start.saturating_add(segment.len());
        let content = raw.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let parts = content.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            return Err(malformed_failure(
                GithubDiagnosticKind::MissingCodeowner,
                span,
            ));
        }
        scalar_within_limit(parts[0], options, span)?;
        for owner in &parts[1..] {
            scalar_within_limit(owner, options, span)?;
        }
        entries.push(CodeownerEntry {
            pattern: parts[0].to_string(),
            owners: parts[1..]
                .iter()
                .map(|owner| (*owner).to_string())
                .collect(),
            span,
        });
        if entries.len() > options.max_nodes {
            return Err(crate::limit_failure(
                seiri_core::StructuredBudgetKind::Nodes,
                span,
            ));
        }
    }
    check_diagnostics(&diagnostics, options, SourceSpan::new(1, 1, 0, 0))?;
    Ok((Codeowners { entries }, diagnostics))
}

fn strip_line_ending(segment: &str) -> &str {
    let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
    without_lf.strip_suffix('\r').unwrap_or(without_lf)
}
