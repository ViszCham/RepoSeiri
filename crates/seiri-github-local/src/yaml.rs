use crate::{
    malformed_failure, scalar_within_limit, unsupported_failure, ParseFailure,
    StructuredParseOptions,
};
use seiri_core::{
    ActionReference, ActionReferenceKind, DependencyBot, DependencyBotProvider, DependencyUpdate,
    GithubDiagnostic, GithubDiagnosticKind, IssueForm, IssueFormField, IssueFormFieldKind,
    IssueFormRequiredFields, IssueRouteCandidate, IssueRouteCandidateKind, PermissionEntry,
    PermissionSet, SourceSpan, StaticUnknownReason, StaticValue, TokenPermission, Workflow,
    WorkflowJob, WorkflowJobCandidate, WorkflowJobCandidateKind, WorkflowStep, WorkflowTrigger,
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
    let mut diagnostics = Vec::new();
    let allowed_top_level = [
        "name",
        "description",
        "body",
        "assignees",
        "labels",
        "title",
        "type",
        "projects",
    ];
    let unknown_top_level_keys = lines
        .iter()
        .filter(|line| !line.is_list && line.indent == 0)
        .filter_map(|line| line.key.as_ref().map(|key| (key, line.span)))
        .filter(|(key, _)| !allowed_top_level.contains(&key.as_str()))
        .map(|(key, span)| {
            diagnostics.push(GithubDiagnostic {
                kind: GithubDiagnosticKind::UnknownField,
                span,
            });
            key.clone()
        })
        .collect::<Vec<_>>();
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
                let unknown_keys = unknown_issue_field_keys(&lines[index + 1..field_end]);
                if kind == IssueFormFieldKind::Unknown || !unknown_keys.is_empty() {
                    diagnostics.push(GithubDiagnostic {
                        kind: GithubDiagnosticKind::UnknownField,
                        span: line.span,
                    });
                }
                fields.push(IssueFormField {
                    kind,
                    id,
                    required,
                    span: line.span,
                    unknown_keys,
                });
                index = field_end;
            } else {
                index += 1;
            }
        }
    }
    let required_fields = IssueFormRequiredFields {
        name: root_key_index(&lines, "name").is_some(),
        description: root_key_index(&lines, "description").is_some(),
        body: root_key_index(&lines, "body").is_some(),
    };
    if !required_fields.name || !required_fields.description || !required_fields.body {
        diagnostics.push(GithubDiagnostic {
            kind: GithubDiagnosticKind::MissingRequiredField,
            span: SourceSpan::new(1, 1, 0, 0),
        });
    }
    let route_candidates = issue_route_candidates(&lines);
    Ok((
        IssueForm {
            name,
            description,
            fields,
            required_fields,
            unknown_top_level_keys,
            route_candidates,
            schema: "github.issue-form.public-preview.2026-07".to_string(),
        },
        diagnostics,
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
                    value: static_string(&trigger, on.span),
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
                        value: line.value.as_deref().map_or(
                            StaticValue::Unknown(StaticUnknownReason::Omitted),
                            |value| static_string(value, line.span),
                        ),
                        span: line.span,
                    });
                } else if line.is_list {
                    if let Some(value) = &line.value {
                        triggers.push(WorkflowTrigger {
                            name: value.clone(),
                            value: static_string(value, line.span),
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
        let mut index = jobs_index + 1;
        while index < end {
            let line = &lines[index];
            if !line.is_list && line.indent == direct_indent {
                if let Some(id) = &line.key {
                    let job_end = next_mapping_end(&lines, index + 1, end, direct_indent);
                    jobs.push(parse_workflow_job(
                        id,
                        line.span,
                        &lines[index + 1..job_end],
                    ));
                    index = job_end;
                    continue;
                }
            }
            index += 1;
        }
    }
    Ok((
        Workflow {
            name,
            triggers,
            jobs,
            permissions: parse_permissions(&lines, 0),
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
                    ecosystem_value: line.value.as_deref().map_or(
                        StaticValue::Unknown(StaticUnknownReason::Omitted),
                        |value| static_string(value, line.span),
                    ),
                    directory_values: dependency_directories(&lines[index + 1..update_end]),
                    schedule_value: field_static(&lines[index + 1..update_end], "interval"),
                    open_pull_requests_limit: field_u32_static(
                        &lines[index + 1..update_end],
                        "open-pull-requests-limit",
                    ),
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

fn parse_workflow_job(id: &str, span: SourceSpan, lines: &[YamlLine]) -> WorkflowJob {
    let name = lines
        .iter()
        .find(|line| !line.is_list && line.key.as_deref() == Some("name"))
        .and_then(|line| {
            line.value
                .as_deref()
                .map(|value| static_string(value, line.span))
        });
    let reusable_workflow = lines
        .iter()
        .find(|line| !line.is_list && line.key.as_deref() == Some("uses"))
        .map(action_reference);
    let steps = parse_steps(lines);
    let mut haystack = id.to_ascii_lowercase();
    if let Some(StaticValue::Literal(name)) = &name {
        haystack.push(' ');
        haystack.push_str(&name.to_ascii_lowercase());
    }
    for line in lines {
        if matches!(line.key.as_deref(), Some("run" | "uses" | "name")) {
            if let Some(value) = &line.value {
                haystack.push(' ');
                haystack.push_str(&value.to_ascii_lowercase());
            }
        }
    }
    WorkflowJob {
        id: id.to_string(),
        span,
        name,
        permissions: parse_permissions(lines, 4),
        reusable_workflow,
        steps,
        candidates: classify_job(&haystack, span),
    }
}

fn parse_steps(lines: &[YamlLine]) -> Vec<WorkflowStep> {
    let Some(steps_index) = lines
        .iter()
        .position(|line| !line.is_list && line.key.as_deref() == Some("steps"))
    else {
        return Vec::new();
    };
    let parent_indent = lines[steps_index].indent;
    let end = section_end(lines, steps_index + 1, parent_indent);
    let mut steps = Vec::new();
    let mut index = steps_index + 1;
    while index < end {
        let line = &lines[index];
        if !line.is_list || line.indent != parent_indent.saturating_add(2) {
            index += 1;
            continue;
        }
        let step_end = lines[index + 1..end]
            .iter()
            .position(|candidate| candidate.is_list && candidate.indent == line.indent)
            .map_or(end, |relative| index + 1 + relative);
        let body = &lines[index..step_end];
        let name = field_line(body, "name").and_then(|line| {
            line.value
                .as_deref()
                .map(|value| static_string(value, line.span))
        });
        let uses = field_line(body, "uses").map(action_reference);
        let has_run_script = field_line(body, "run").is_some();
        steps.push(WorkflowStep {
            name,
            uses,
            has_run_script,
            span: line.span,
        });
        index = step_end;
    }
    steps
}

fn field_line<'a>(lines: &'a [YamlLine], key: &str) -> Option<&'a YamlLine> {
    lines.iter().find(|line| line.key.as_deref() == Some(key))
}

fn parse_permissions(lines: &[YamlLine], expected_indent: usize) -> PermissionSet {
    let Some(index) = lines.iter().position(|line| {
        !line.is_list
            && line.indent == expected_indent
            && line.key.as_deref() == Some("permissions")
    }) else {
        return PermissionSet::default();
    };
    let line = &lines[index];
    if let Some(value) = line.value.as_deref() {
        return PermissionSet {
            default: match value {
                "read-all" => TokenPermission::Read,
                "write-all" => TokenPermission::Write,
                "{}" => TokenPermission::None,
                _ => TokenPermission::DefaultOrInheritedUnknown,
            },
            entries: if contains_expression(value) {
                vec![PermissionEntry {
                    scope: "*".to_string(),
                    permission: StaticValue::Expression { span: line.span },
                    span: line.span,
                }]
            } else {
                Vec::new()
            },
            span: Some(line.span),
        };
    }
    let end = section_end(lines, index + 1, line.indent);
    let entries = lines[index + 1..end]
        .iter()
        .filter(|entry| !entry.is_list && entry.indent == line.indent.saturating_add(2))
        .filter_map(|entry| {
            entry.key.as_ref().map(|scope| PermissionEntry {
                scope: scope.clone(),
                permission: entry.value.as_deref().map_or(
                    StaticValue::Unknown(StaticUnknownReason::Omitted),
                    |value| permission_value(value, entry.span),
                ),
                span: entry.span,
            })
        })
        .collect();
    PermissionSet {
        default: TokenPermission::None,
        entries,
        span: Some(line.span),
    }
}

fn permission_value(value: &str, span: SourceSpan) -> StaticValue<TokenPermission> {
    if contains_expression(value) {
        StaticValue::Expression { span }
    } else {
        match value {
            "none" => StaticValue::Literal(TokenPermission::None),
            "read" => StaticValue::Literal(TokenPermission::Read),
            "write" => StaticValue::Literal(TokenPermission::Write),
            _ => StaticValue::Unsupported { span },
        }
    }
}

fn action_reference(line: &YamlLine) -> ActionReference {
    let Some(value) = line.value.as_deref() else {
        return ActionReference {
            raw: StaticValue::Unknown(StaticUnknownReason::Omitted),
            kind: ActionReferenceKind::Malformed,
            span: line.span,
        };
    };
    let raw = static_string(value, line.span);
    let kind = if contains_expression(value) {
        ActionReferenceKind::Dynamic
    } else if value.starts_with("./") {
        ActionReferenceKind::LocalPath(value.to_string())
    } else if let Some(image) = value.strip_prefix("docker://") {
        ActionReferenceKind::Docker(image.to_string())
    } else if let Some((_, reference)) = value.rsplit_once('@') {
        if reference.len() == 40 && reference.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            ActionReferenceKind::FullObjectId(reference.to_ascii_lowercase())
        } else if reference.is_empty() {
            ActionReferenceKind::Malformed
        } else {
            ActionReferenceKind::TagOrBranch(reference.to_string())
        }
    } else {
        ActionReferenceKind::Malformed
    };
    ActionReference {
        raw,
        kind,
        span: line.span,
    }
}

fn classify_job(value: &str, span: SourceSpan) -> Vec<WorkflowJobCandidate> {
    let classes = [
        (
            WorkflowJobCandidateKind::Test,
            ["test", "cargo test", "pytest"].as_slice(),
        ),
        (
            WorkflowJobCandidateKind::Build,
            ["build", "compile"].as_slice(),
        ),
        (
            WorkflowJobCandidateKind::Lint,
            ["lint", "clippy", "fmt"].as_slice(),
        ),
        (
            WorkflowJobCandidateKind::Documentation,
            ["docs", "documentation", "mdbook"].as_slice(),
        ),
        (
            WorkflowJobCandidateKind::Release,
            ["release", "publish"].as_slice(),
        ),
        (
            WorkflowJobCandidateKind::Security,
            ["security", "audit", "codeql", "dependency-review"].as_slice(),
        ),
        (
            WorkflowJobCandidateKind::Deploy,
            ["deploy", "deployment", "pages"].as_slice(),
        ),
    ];
    classes
        .into_iter()
        .filter(|(_, needles)| needles.iter().any(|needle| value.contains(needle)))
        .map(|(kind, _)| WorkflowJobCandidate { kind, span })
        .collect()
}

fn static_string(value: &str, span: SourceSpan) -> StaticValue<String> {
    if contains_expression(value) {
        StaticValue::Expression { span }
    } else {
        StaticValue::Literal(value.to_string())
    }
}

fn contains_expression(value: &str) -> bool {
    value.contains("${{")
}

fn field_static(lines: &[YamlLine], key: &str) -> StaticValue<String> {
    field_line(lines, key).map_or(StaticValue::Unknown(StaticUnknownReason::Omitted), |line| {
        line.value.as_deref().map_or(
            StaticValue::Unknown(StaticUnknownReason::Omitted),
            |value| static_string(value, line.span),
        )
    })
}

fn field_u32_static(lines: &[YamlLine], key: &str) -> StaticValue<u32> {
    let Some(line) = field_line(lines, key) else {
        return StaticValue::Unknown(StaticUnknownReason::Omitted);
    };
    let Some(value) = line.value.as_deref() else {
        return StaticValue::Unknown(StaticUnknownReason::Omitted);
    };
    if contains_expression(value) {
        StaticValue::Expression { span: line.span }
    } else {
        value.parse::<u32>().map_or(
            StaticValue::Unsupported { span: line.span },
            StaticValue::Literal,
        )
    }
}

fn dependency_directories(lines: &[YamlLine]) -> Vec<StaticValue<String>> {
    if let Some(line) = field_line(lines, "directory") {
        return line
            .value
            .as_deref()
            .map(|value| vec![static_string(value, line.span)])
            .unwrap_or_default();
    }
    let Some(index) = lines
        .iter()
        .position(|line| line.key.as_deref() == Some("directories"))
    else {
        return Vec::new();
    };
    let end = section_end(lines, index + 1, lines[index].indent);
    lines[index + 1..end]
        .iter()
        .filter(|line| line.is_list)
        .filter_map(|line| {
            line.value
                .as_deref()
                .map(|value| static_string(value, line.span))
        })
        .collect()
}

fn unknown_issue_field_keys(lines: &[YamlLine]) -> Vec<String> {
    let allowed = ["id", "attributes", "validations"];
    lines
        .iter()
        .filter(|line| line.indent <= 4)
        .filter_map(|line| line.key.as_ref())
        .filter(|key| !allowed.contains(&key.as_str()))
        .cloned()
        .collect()
}

fn issue_route_candidates(lines: &[YamlLine]) -> Vec<IssueRouteCandidate> {
    let mut candidates = Vec::new();
    for (kind, words) in [
        (
            IssueRouteCandidateKind::Security,
            ["security", "vulnerability", "cve"].as_slice(),
        ),
        (
            IssueRouteCandidateKind::Question,
            ["question", "help", "support"].as_slice(),
        ),
    ] {
        if let Some(span) = lines
            .iter()
            .filter(|line| {
                matches!(
                    line.key.as_deref(),
                    Some("name" | "description" | "labels" | "type" | "id")
                )
            })
            .find_map(|line| {
                line.value.as_ref().and_then(|value| {
                    let value = value.to_ascii_lowercase();
                    words
                        .iter()
                        .any(|word| value.contains(word))
                        .then_some(line.span)
                })
            })
        {
            candidates.push(IssueRouteCandidate { kind, span });
        }
    }
    candidates
}

fn next_mapping_end(lines: &[YamlLine], start: usize, end: usize, indent: usize) -> usize {
    lines[start..end]
        .iter()
        .position(|line| !line.is_list && line.indent == indent)
        .map_or(end, |relative| start + relative)
}

fn tokenize(source: &str, options: &StructuredParseOptions) -> Result<Vec<YamlLine>, ParseFailure> {
    let mut lines = Vec::new();
    let mut byte_start = 0usize;
    let mut block_parent_indent = None;
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
        if let Some(parent_indent) = block_parent_indent {
            if first > parent_indent {
                continue;
            }
            block_parent_indent = None;
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
                if value
                    .as_deref()
                    .is_some_and(|value| value.starts_with('|') || value.starts_with('>'))
                {
                    block_parent_indent = Some(indent);
                }
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
    if value.starts_with('&') || value.starts_with('*') {
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
        Some("upload") => IssueFormFieldKind::Upload,
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
