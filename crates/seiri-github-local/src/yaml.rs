use crate::{
    malformed_failure, scalar_within_limit, unsupported_failure, ParseFailure,
    StructuredParseOptions,
};
use seiri_core::{
    DependencyBot, DependencyBotProvider, DependencyUpdate, GithubDiagnostic, GithubDiagnosticKind,
    IssueForm, IssueFormField, IssueFormFieldKind, SourceSpan, Workflow, WorkflowJob,
    WorkflowTrigger,
};

#[derive(Debug, Clone)]
struct YamlLine {
    indent: usize,
    is_list: bool,
    key: Option<String>,
    value: Option<String>,
    span: SourceSpan,
}

pub(crate) fn parse_issue_form(
    source: &str,
    options: &StructuredParseOptions,
) -> Result<(IssueForm, Vec<GithubDiagnostic>), ParseFailure> {
    let lines = tokenize(source, options)?;
    let name = root_scalar(&lines, "name");
    let description = root_scalar(&lines, "description");
    let mut fields = Vec::new();
    if let Some(body_index) = root_key_index(&lines, "body") {
        let body_indent = lines[body_index].indent;
        let end = section_end(&lines, body_index + 1, body_indent);
        let mut index = body_index + 1;
        while index < end {
            let line = &lines[index];
            if line.is_list && line.key.as_deref() == Some("type") {
                let field_end = next_field_end(&lines, index + 1, end, line.indent);
                let kind = issue_form_field_kind(line.value.as_deref());
                let id = field_scalar(&lines[index + 1..field_end], "id");
                let required = field_bool(&lines[index + 1..field_end], "required")?;
                fields.push(IssueFormField {
                    kind,
                    id,
                    required,
                    span: line.span,
                });
                index = field_end;
            } else {
                index += 1;
            }
        }
    }
    Ok((
        IssueForm {
            name,
            description,
            fields,
        },
        Vec::new(),
    ))
}

pub(crate) fn parse_workflow(
    source: &str,
    options: &StructuredParseOptions,
) -> Result<(Workflow, Vec<GithubDiagnostic>), ParseFailure> {
    let lines = tokenize(source, options)?;
    let name = root_scalar(&lines, "name");
    let mut triggers = Vec::new();
    if let Some(on_index) = root_key_index(&lines, "on") {
        let on = &lines[on_index];
        if let Some(value) = &on.value {
            for trigger in scalar_list(value) {
                triggers.push(WorkflowTrigger {
                    name: trigger,
                    span: on.span,
                });
            }
        }
        let end = section_end(&lines, on_index + 1, on.indent);
        let direct_indent = on.indent.saturating_add(2);
        for line in &lines[on_index + 1..end] {
            if line.indent == direct_indent {
                if let Some(key) = &line.key {
                    triggers.push(WorkflowTrigger {
                        name: key.clone(),
                        span: line.span,
                    });
                } else if line.is_list {
                    if let Some(value) = &line.value {
                        triggers.push(WorkflowTrigger {
                            name: value.clone(),
                            span: line.span,
                        });
                    }
                }
            }
        }
    }

    let mut jobs = Vec::new();
    if let Some(jobs_index) = root_key_index(&lines, "jobs") {
        let jobs_line = &lines[jobs_index];
        let end = section_end(&lines, jobs_index + 1, jobs_line.indent);
        let direct_indent = jobs_line.indent.saturating_add(2);
        for line in &lines[jobs_index + 1..end] {
            if !line.is_list && line.indent == direct_indent {
                if let Some(id) = &line.key {
                    jobs.push(WorkflowJob {
                        id: id.clone(),
                        span: line.span,
                    });
                }
            }
        }
    }
    Ok((
        Workflow {
            name,
            triggers,
            jobs,
        },
        Vec::new(),
    ))
}

pub(crate) fn parse_dependabot(
    source: &str,
    options: &StructuredParseOptions,
) -> Result<(DependencyBot, Vec<GithubDiagnostic>), ParseFailure> {
    let lines = tokenize(source, options)?;
    let mut updates = Vec::new();
    if let Some(updates_index) = root_key_index(&lines, "updates") {
        let updates_line = &lines[updates_index];
        let end = section_end(&lines, updates_index + 1, updates_line.indent);
        let mut index = updates_index + 1;
        while index < end {
            let line = &lines[index];
            if line.is_list && line.key.as_deref() == Some("package-ecosystem") {
                let update_end = next_field_end(&lines, index + 1, end, line.indent);
                updates.push(DependencyUpdate {
                    ecosystem: line.value.clone(),
                    directory: field_scalar(&lines[index + 1..update_end], "directory"),
                    schedule: field_scalar(&lines[index + 1..update_end], "interval"),
                    span: line.span,
                });
                index = update_end;
            } else {
                index += 1;
            }
        }
    }
    Ok((
        DependencyBot {
            provider: DependencyBotProvider::Dependabot,
            updates,
        },
        Vec::new(),
    ))
}

fn tokenize(source: &str, options: &StructuredParseOptions) -> Result<Vec<YamlLine>, ParseFailure> {
    let mut lines = Vec::new();
    let mut byte_start = 0usize;
    for (line_index, segment) in source.split_inclusive('\n').enumerate() {
        let raw = strip_line_ending(segment);
        let line_number = line_index + 1;
        let first = raw
            .as_bytes()
            .iter()
            .position(|byte| *byte != b' ')
            .unwrap_or(raw.len());
        let span = SourceSpan::new(line_number, 1, byte_start, byte_start + raw.len());
        byte_start = byte_start.saturating_add(segment.len());
        if raw[first..].is_empty() || raw[first..].starts_with('#') {
            continue;
        }
        if raw[..first].contains('\t') || raw[first..].starts_with('\t') || first % 2 != 0 {
            return Err(unsupported_failure(span));
        }
        let indent = first;
        if indent / 2 > options.max_depth {
            return Err(crate::limit_failure(
                seiri_core::StructuredBudgetKind::Depth,
                span,
            ));
        }
        let mut rest = &raw[first..];
        let mut is_list = false;
        if let Some(value) = rest.strip_prefix("- ") {
            is_list = true;
            rest = value;
        } else if rest == "-" {
            return Err(unsupported_failure(span));
        }
        let (key, value) = match rest.split_once(':') {
            Some((key, value)) if valid_key(key.trim()) => {
                let value = normalize_scalar(value.trim(), options, span)?;
                (Some(key.trim().to_string()), value)
            }
            Some(_) => return Err(unsupported_failure(span)),
            None if is_list => {
                let value = normalize_scalar(rest.trim(), options, span)?;
                (None, value)
            }
            None => return Err(unsupported_failure(span)),
        };
        lines.push(YamlLine {
            indent,
            is_list,
            key,
            value,
            span,
        });
        if lines.len() > options.max_nodes {
            return Err(crate::limit_failure(
                seiri_core::StructuredBudgetKind::Nodes,
                span,
            ));
        }
    }
    Ok(lines)
}

fn valid_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn normalize_scalar(
    value: &str,
    options: &StructuredParseOptions,
    span: SourceSpan,
) -> Result<Option<String>, ParseFailure> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.starts_with('&')
        || value.starts_with('*')
        || value.starts_with('|')
        || value.starts_with('>')
    {
        return Err(unsupported_failure(span));
    }
    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value);
    scalar_within_limit(value, options, span)?;
    Ok(Some(value.to_string()))
}

fn root_key_index(lines: &[YamlLine], key: &str) -> Option<usize> {
    lines
        .iter()
        .position(|line| !line.is_list && line.indent == 0 && line.key.as_deref() == Some(key))
}

fn root_scalar(lines: &[YamlLine], key: &str) -> Option<String> {
    root_key_index(lines, key).and_then(|index| lines[index].value.clone())
}

fn field_scalar(lines: &[YamlLine], key: &str) -> Option<String> {
    lines
        .iter()
        .find(|line| line.key.as_deref() == Some(key))
        .and_then(|line| line.value.clone())
}

fn field_bool(lines: &[YamlLine], key: &str) -> Result<Option<bool>, ParseFailure> {
    let Some(line) = lines.iter().find(|line| line.key.as_deref() == Some(key)) else {
        return Ok(None);
    };
    match line.value.as_deref() {
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) | None => Err(malformed_failure(
            GithubDiagnosticKind::MalformedValue,
            line.span,
        )),
    }
}

fn section_end(lines: &[YamlLine], start: usize, parent_indent: usize) -> usize {
    lines[start..]
        .iter()
        .position(|line| !line.is_list && line.indent <= parent_indent)
        .map_or(lines.len(), |relative| start + relative)
}

fn next_field_end(lines: &[YamlLine], start: usize, end: usize, field_indent: usize) -> usize {
    lines[start..end]
        .iter()
        .position(|line| {
            line.is_list && line.indent == field_indent && line.key.as_deref() == Some("type")
        })
        .map_or(end, |relative| start + relative)
}

fn issue_form_field_kind(value: Option<&str>) -> IssueFormFieldKind {
    match value {
        Some("input") => IssueFormFieldKind::Input,
        Some("textarea") => IssueFormFieldKind::Textarea,
        Some("dropdown") => IssueFormFieldKind::Dropdown,
        Some("checkboxes") => IssueFormFieldKind::Checkboxes,
        Some("markdown") => IssueFormFieldKind::Markdown,
        _ => IssueFormFieldKind::Unknown,
    }
}

fn scalar_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn strip_line_ending(segment: &str) -> &str {
    let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
    without_lf.strip_suffix('\r').unwrap_or(without_lf)
}
