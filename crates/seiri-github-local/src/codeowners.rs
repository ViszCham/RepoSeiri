use crate::{
    check_diagnostics, malformed_failure, scalar_within_limit, ParseFailure, StructuredParseOptions,
};
use seiri_core::{CodeownerEntry, Codeowners, GithubDiagnostic, GithubDiagnosticKind, SourceSpan};
use seiri_core::{CodeownersOp, CodeownersPatternProgram, CodeownersSkippedLine};

pub(crate) fn parse_codeowners(
    source: &str,
    options: &StructuredParseOptions,
) -> Result<(Codeowners, Vec<GithubDiagnostic>), ParseFailure> {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut skipped = Vec::new();
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
        let program = match compile_pattern(parts[0], &parts[1..], span, options) {
            Ok(program) => program,
            Err(kind) => {
                diagnostics.push(GithubDiagnostic { kind, span });
                skipped.push(CodeownersSkippedLine {
                    pattern: parts[0].to_string(),
                    span,
                    diagnostic: kind,
                });
                check_diagnostics(&diagnostics, options, span)?;
                continue;
            }
        };
        entries.push(CodeownerEntry {
            pattern: parts[0].to_string(),
            owners: parts[1..]
                .iter()
                .map(|owner| (*owner).to_string())
                .collect(),
            span,
            program,
        });
        if entries.len() > options.max_nodes {
            return Err(crate::limit_failure(
                seiri_core::StructuredBudgetKind::Nodes,
                span,
            ));
        }
    }
    check_diagnostics(&diagnostics, options, SourceSpan::new(1, 1, 0, 0))?;
    Ok((Codeowners { entries, skipped }, diagnostics))
}

fn compile_pattern(
    pattern: &str,
    owners: &[&str],
    span: SourceSpan,
    options: &StructuredParseOptions,
) -> Result<CodeownersPatternProgram, GithubDiagnosticKind> {
    if pattern.starts_with('!')
        || pattern.starts_with("\\#")
        || pattern.contains('[')
        || pattern.contains(']')
        || pattern.contains('\\')
    {
        return Err(GithubDiagnosticKind::UnsupportedCodeownersPattern);
    }
    let mut ops = Vec::new();
    let mut literal = String::new();
    let bytes = pattern.as_bytes();
    let mut index = 0usize;
    if bytes.first() == Some(&b'/') {
        ops.push(CodeownersOp::Root);
        index = 1;
    }
    while index < bytes.len() {
        match bytes[index] {
            b'/' => {
                flush_literal(&mut literal, &mut ops);
                ops.push(CodeownersOp::Slash);
                index += 1;
            }
            b'*' if bytes.get(index + 1) == Some(&b'*') => {
                flush_literal(&mut literal, &mut ops);
                ops.push(CodeownersOp::DoubleStar);
                index += 2;
            }
            b'*' => {
                flush_literal(&mut literal, &mut ops);
                ops.push(CodeownersOp::Star);
                index += 1;
            }
            byte if byte.is_ascii() => {
                literal.push(char::from(byte));
                index += 1;
            }
            _ => return Err(GithubDiagnosticKind::UnsupportedCodeownersPattern),
        }
        if ops.len() > options.max_nodes.min(256) {
            return Err(GithubDiagnosticKind::BudgetExceeded(
                seiri_core::StructuredBudgetKind::Nodes,
            ));
        }
    }
    flush_literal(&mut literal, &mut ops);
    if ops.is_empty() || owners.len() > 64 {
        return Err(GithubDiagnosticKind::BudgetExceeded(
            seiri_core::StructuredBudgetKind::Nodes,
        ));
    }
    Ok(CodeownersPatternProgram {
        ops,
        owners: owners.iter().map(|owner| (*owner).to_string()).collect(),
        source: span,
    })
}

fn flush_literal(literal: &mut String, ops: &mut Vec<CodeownersOp>) {
    if !literal.is_empty() {
        ops.push(CodeownersOp::Literal(std::mem::take(literal)));
    }
}

fn strip_line_ending(segment: &str) -> &str {
    let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
    without_lf.strip_suffix('\r').unwrap_or(without_lf)
}
