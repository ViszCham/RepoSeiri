use crate::{
    limit_failure, malformed_failure, scalar_within_limit, ParseFailure, StructuredParseOptions,
};
use seiri_core::{
    DependencyBot, DependencyBotProvider, DependencyUpdate, GithubDiagnostic, GithubDiagnosticKind,
    SourceSpan, StaticUnknownReason, StaticValue, StructuredBudgetKind,
};
use serde_json::Value;

pub(crate) fn parse_renovate(
    source: &str,
    options: &StructuredParseOptions,
) -> Result<(DependencyBot, Vec<GithubDiagnostic>), ParseFailure> {
    let value: Value = serde_json::from_str(source).map_err(|error| {
        malformed_failure(
            GithubDiagnosticKind::MalformedValue,
            span_for_line_column(source, error.line(), error.column()),
        )
    })?;
    let mut nodes = 0usize;
    validate_json_value(&value, 0, &mut nodes, options, empty_span())?;
    let updates = value
        .get("packageRules")
        .and_then(Value::as_array)
        .map(|rules| {
            rules
                .iter()
                .filter_map(Value::as_object)
                .map(|rule| DependencyUpdate {
                    ecosystem: string_or_first_array(rule.get("matchManagers")),
                    directory: string_or_first_array(rule.get("matchPaths")),
                    schedule: string_or_first_array(rule.get("schedule")),
                    span: empty_span(),
                    ecosystem_value: json_static_string(rule.get("matchManagers")),
                    directory_values: json_static_strings(rule.get("matchPaths")),
                    schedule_value: json_static_string(rule.get("schedule")),
                    open_pull_requests_limit: json_static_u32(
                        rule.get("prConcurrentLimit")
                            .or_else(|| rule.get("prHourlyLimit")),
                    ),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok((
        DependencyBot {
            provider: DependencyBotProvider::Renovate,
            updates,
        },
        Vec::new(),
    ))
}

fn validate_json_value(
    value: &Value,
    depth: usize,
    nodes: &mut usize,
    options: &StructuredParseOptions,
    span: SourceSpan,
) -> Result<(), ParseFailure> {
    *nodes = nodes.saturating_add(1);
    if *nodes > options.max_nodes {
        return Err(limit_failure(StructuredBudgetKind::Nodes, span));
    }
    if depth > options.max_depth {
        return Err(limit_failure(StructuredBudgetKind::Depth, span));
    }
    match value {
        Value::String(value) => scalar_within_limit(value, options, span),
        Value::Array(values) => {
            for value in values {
                validate_json_value(value, depth + 1, nodes, options, span)?;
            }
            Ok(())
        }
        Value::Object(entries) => {
            for (key, value) in entries {
                scalar_within_limit(key, options, span)?;
                validate_json_value(value, depth + 1, nodes, options, span)?;
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
    }
}

fn string_or_first_array(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Array(values)) => values.iter().find_map(Value::as_str).map(ToOwned::to_owned),
        Some(Value::Null | Value::Bool(_) | Value::Number(_) | Value::Object(_)) | None => None,
    }
}

fn json_static_string(value: Option<&Value>) -> StaticValue<String> {
    match value {
        Some(Value::String(value)) if value.contains("{{") => {
            StaticValue::Expression { span: empty_span() }
        }
        Some(Value::String(value)) => StaticValue::Literal(value.clone()),
        Some(Value::Array(values)) => values.iter().find_map(Value::as_str).map_or(
            StaticValue::Unsupported { span: empty_span() },
            |value| {
                if value.contains("{{") {
                    StaticValue::Expression { span: empty_span() }
                } else {
                    StaticValue::Literal(value.to_string())
                }
            },
        ),
        Some(_) => StaticValue::Unsupported { span: empty_span() },
        None => StaticValue::Unknown(StaticUnknownReason::Omitted),
    }
}

fn json_static_strings(value: Option<&Value>) -> Vec<StaticValue<String>> {
    match value {
        Some(Value::String(value)) => vec![json_static_string(Some(&Value::String(value.clone())))],
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| json_static_string(Some(value)))
            .collect(),
        Some(value) => vec![json_static_string(Some(value))],
        None => Vec::new(),
    }
}

fn json_static_u32(value: Option<&Value>) -> StaticValue<u32> {
    match value {
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map_or(
                StaticValue::Unsupported { span: empty_span() },
                StaticValue::Literal,
            ),
        Some(Value::String(value)) if value.contains("{{") => {
            StaticValue::Expression { span: empty_span() }
        }
        Some(_) => StaticValue::Unsupported { span: empty_span() },
        None => StaticValue::Unknown(StaticUnknownReason::Omitted),
    }
}

fn span_for_line_column(source: &str, line: usize, column: usize) -> SourceSpan {
    let line = line.max(1);
    let column = column.max(1);
    let mut byte_start = 0usize;
    for (current_line, segment) in (1usize..).zip(source.split_inclusive('\n')) {
        if current_line == line {
            let offset = segment
                .char_indices()
                .nth(column.saturating_sub(1))
                .map(|(index, _)| index)
                .unwrap_or(segment.len());
            return SourceSpan::new(line, column, byte_start + offset, byte_start + offset);
        }
        byte_start = byte_start.saturating_add(segment.len());
    }
    SourceSpan::new(line, column, source.len(), source.len())
}

const fn empty_span() -> SourceSpan {
    SourceSpan::new(1, 1, 0, 0)
}
