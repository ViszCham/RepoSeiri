use seiri_core::{
    MarkdownBadge, MarkdownHeading, MarkdownLink, ReadmeSummary, RouteCandidate, RouteKind,
    RouteSource, SourceSpan,
};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod route_map;

use route_map::build_route_map;

#[derive(Debug)]
pub enum MarkdownError {
    Io { path: PathBuf, source: io::Error },
}

impl Display for MarkdownError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read markdown {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for MarkdownError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
        }
    }
}

pub fn analyze_readme(repo_root: impl AsRef<Path>) -> Result<Option<ReadmeSummary>, MarkdownError> {
    let repo_root = repo_root.as_ref();
    let Some(readme_path) = find_readme(repo_root) else {
        return Ok(None);
    };
    let text = fs::read_to_string(&readme_path).map_err(|source| MarkdownError::Io {
        path: readme_path.clone(),
        source,
    })?;
    Ok(Some(parse_readme_with_context(
        normalize_relative_path(repo_root, &readme_path),
        &text,
        Some(repo_root),
    )))
}

pub fn parse_readme(path: impl Into<String>, text: &str) -> ReadmeSummary {
    parse_readme_with_context(path, text, None)
}

fn parse_readme_with_context(
    path: impl Into<String>,
    text: &str,
    repo_root: Option<&Path>,
) -> ReadmeSummary {
    let mut headings = Vec::new();
    let mut links = Vec::new();
    let mut badges = Vec::new();
    let mut route_candidates = Vec::new();
    let path = path.into();

    for line in markdown_lines(text) {
        if let Some(heading) = parse_heading(line) {
            let route = classify_route(&heading.text, None);
            if route != RouteKind::Unknown {
                route_candidates.push(RouteCandidate {
                    route,
                    source: RouteSource::Heading,
                    text: heading.text.clone(),
                    target: None,
                    line: line.number,
                    span: heading.span,
                });
            }
            headings.push(heading);
        }

        for image in parse_markdown_links(line, true) {
            if looks_like_badge(&image.text, &image.target) {
                route_candidates.push(RouteCandidate {
                    route: RouteKind::Automation,
                    source: RouteSource::Badge,
                    text: image.text.clone(),
                    target: Some(image.target.clone()),
                    line: line.number,
                    span: image.span,
                });
                badges.push(MarkdownBadge {
                    alt: image.text,
                    target: image.target,
                    line: line.number,
                    span: image.span,
                });
            }
        }

        for link in parse_markdown_links(line, false) {
            let route = classify_route(&link.text, Some(&link.target));
            if route != RouteKind::Unknown {
                route_candidates.push(RouteCandidate {
                    route,
                    source: RouteSource::Link,
                    text: link.text.clone(),
                    target: Some(link.target.clone()),
                    line: line.number,
                    span: link.span,
                });
            }
            links.push(MarkdownLink {
                route: (route != RouteKind::Unknown).then_some(route),
                ..link
            });
        }
    }

    route_candidates.sort_by(|left, right| {
        left.route
            .cmp(&right.route)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.text.cmp(&right.text))
            .then_with(|| span_start(left.span).cmp(&span_start(right.span)))
    });

    ReadmeSummary {
        route_map: build_route_map(&route_candidates, repo_root),
        path,
        headings,
        links,
        badges,
        route_candidates,
    }
}

pub fn classify_route(text: &str, target: Option<&str>) -> RouteKind {
    let text_only = text.to_ascii_lowercase();
    let text_route = classify_route_text(&text_only);
    if text_route != RouteKind::Unknown {
        return text_route;
    }

    let combined = match target {
        Some(target) => format!("{text} {target}").to_ascii_lowercase(),
        None => text.to_ascii_lowercase(),
    };

    if is_hygiene_route_text(&combined) {
        RouteKind::Hygiene
    } else if contains_any(&combined, &["docs", "documentation", "guide", "manual"]) {
        RouteKind::Docs
    } else if contains_any(
        &combined,
        &[
            "quickstart",
            "quick start",
            "getting started",
            "install",
            "usage",
            "example",
        ],
    ) {
        RouteKind::Quickstart
    } else if is_intake_route_text(&combined) {
        RouteKind::Intake
    } else if is_lifecycle_route_text(&combined) {
        RouteKind::Lifecycle
    } else if contains_any(
        &combined,
        &[
            "support",
            "discussion",
            "help",
            "contact",
            "question",
            "issue",
        ],
    ) {
        RouteKind::Support
    } else if contains_any(&combined, &["contributing", "contribute", "development"]) {
        RouteKind::Contributing
    } else if contains_any(&combined, &["security", "vulnerability", "disclosure"]) {
        RouteKind::Security
    } else if contains_any(
        &combined,
        &[
            "release",
            "changelog",
            "changes",
            "version",
            "compatibility",
        ],
    ) {
        RouteKind::Release
    } else if contains_any(&combined, &["governance", "roadmap", "rfc", "proposal"]) {
        RouteKind::Governance
    } else if contains_any(&combined, &["license", "copying"]) {
        RouteKind::License
    } else if contains_any(
        &combined,
        &["codeowners", "maintainer", "ownership", "owner"],
    ) {
        RouteKind::Ownership
    } else if contains_any(&combined, &["workflow", "actions", "ci", "build", "badge"]) {
        RouteKind::Automation
    } else if combined.starts_with('#') || combined.contains("readme") {
        RouteKind::Identity
    } else {
        RouteKind::Unknown
    }
}

fn classify_route_text(value: &str) -> RouteKind {
    if is_hygiene_route_text(value) {
        RouteKind::Hygiene
    } else if contains_any(
        value,
        &[
            "quickstart",
            "quick start",
            "getting started",
            "install",
            "usage",
            "example",
        ],
    ) {
        RouteKind::Quickstart
    } else if contains_any(value, &["docs", "documentation", "guide", "manual"]) {
        RouteKind::Docs
    } else if is_intake_route_text(value) {
        RouteKind::Intake
    } else if is_lifecycle_route_text(value) {
        RouteKind::Lifecycle
    } else if contains_any(
        value,
        &[
            "support",
            "discussion",
            "help",
            "contact",
            "question",
            "issue",
        ],
    ) {
        RouteKind::Support
    } else if contains_any(value, &["contributing", "contribute", "development"]) {
        RouteKind::Contributing
    } else if contains_any(value, &["security", "vulnerability", "disclosure"]) {
        RouteKind::Security
    } else if contains_any(
        value,
        &[
            "release",
            "changelog",
            "changes",
            "version",
            "compatibility",
        ],
    ) {
        RouteKind::Release
    } else if contains_any(value, &["governance", "roadmap", "rfc", "proposal"]) {
        RouteKind::Governance
    } else if contains_any(value, &["license", "copying"]) {
        RouteKind::License
    } else if contains_any(value, &["codeowners", "maintainer", "ownership", "owner"]) {
        RouteKind::Ownership
    } else if contains_any(value, &["workflow", "actions", "ci", "build", "badge"]) {
        RouteKind::Automation
    } else if value.starts_with('#') || value.contains("readme") {
        RouteKind::Identity
    } else {
        RouteKind::Unknown
    }
}

fn is_intake_route_text(value: &str) -> bool {
    contains_any(
        value,
        &[
            "issue template",
            "issue form",
            "bug report",
            "feature request",
            "pull request template",
            "pr template",
            "triage",
            "intake",
        ],
    ) || (contains_any(value, &["issues", "issue"])
        && contains_any(value, &["bug", "feature", "template", "form"]))
}

fn is_lifecycle_route_text(value: &str) -> bool {
    contains_any(
        value,
        &[
            "lifecycle",
            "life cycle",
            "maintenance",
            "maintained",
            "deprecation",
            "deprecated",
            "end of life",
            "end-of-life",
            "eol",
            "lts",
            "long term support",
            "supported versions",
            "version support",
            "support matrix",
            "compatibility policy",
            "archive policy",
            "archival",
            "sunset",
        ],
    )
}

fn is_hygiene_route_text(value: &str) -> bool {
    contains_any(
        value,
        &[
            "hygiene",
            "repository hygiene",
            "cleanup",
            "clean-up",
            "self-audit",
            "self audit",
        ],
    )
}

#[derive(Debug, Clone, Copy)]
struct MarkdownLine<'a> {
    number: usize,
    byte_start: usize,
    text: &'a str,
}

fn markdown_lines(text: &str) -> impl Iterator<Item = MarkdownLine<'_>> {
    let mut number = 0;
    let mut byte_start = 0;
    text.split_inclusive('\n').map(move |segment| {
        number += 1;
        let current_start = byte_start;
        byte_start += segment.len();
        let line = if let Some(line) = segment.strip_suffix('\n') {
            line.strip_suffix('\r').unwrap_or(line)
        } else {
            segment
        };
        MarkdownLine {
            number,
            byte_start: current_start,
            text: line,
        }
    })
}

fn parse_heading(line: MarkdownLine<'_>) -> Option<MarkdownHeading> {
    let marker_start = first_non_whitespace_byte(line.text)?;
    let trimmed = &line.text[marker_start..];
    let level = trimmed.chars().take_while(|value| *value == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..)?;
    if !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim().trim_end_matches('#').trim();
    if text.is_empty() {
        return None;
    }
    Some(MarkdownHeading {
        level: level as u8,
        text: text.to_string(),
        line: line.number,
        span: Some(source_span(line, marker_start, line.text.len())),
    })
}

fn parse_markdown_links(line: MarkdownLine<'_>, images_only: bool) -> Vec<MarkdownLink> {
    let bytes = line.text.as_bytes();
    let mut cursor = 0;
    let mut out = Vec::new();

    while cursor < bytes.len() {
        let is_image =
            bytes[cursor] == b'!' && cursor + 1 < bytes.len() && bytes[cursor + 1] == b'[';
        let starts_link = bytes[cursor] == b'[' || is_image;
        if !starts_link {
            cursor += 1;
            continue;
        }
        if images_only != is_image {
            cursor += if is_image { 2 } else { 1 };
            continue;
        }

        let label_start = cursor + usize::from(is_image) + 1;
        let Some(label_end_offset) = line.text[label_start..].find(']') else {
            cursor += 1;
            continue;
        };
        let label_end = label_start + label_end_offset;
        let open_paren = label_end + 1;
        if bytes.get(open_paren) != Some(&b'(') {
            cursor = label_end + 1;
            continue;
        }
        let target_start = open_paren + 1;
        let Some(target_end_offset) = line.text[target_start..].find(')') else {
            cursor = target_start;
            continue;
        };
        let target_end = target_start + target_end_offset;
        let text = line.text[label_start..label_end].trim();
        let target = line.text[target_start..target_end].trim();
        if !text.is_empty() && !target.is_empty() {
            out.push(MarkdownLink {
                text: text.to_string(),
                target: target.to_string(),
                line: line.number,
                span: Some(source_span(line, cursor, target_end + 1)),
                route: None,
            });
        }
        cursor = target_end + 1;
    }

    out
}

fn first_non_whitespace_byte(line: &str) -> Option<usize> {
    line.char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map(|(index, _)| index)
}

fn source_span(line: MarkdownLine<'_>, start_in_line: usize, end_in_line: usize) -> SourceSpan {
    SourceSpan::new(
        line.number,
        column_for_byte(line.text, start_in_line),
        line.byte_start + start_in_line,
        line.byte_start + end_in_line,
    )
}

fn column_for_byte(line: &str, byte_index: usize) -> usize {
    line[..byte_index].chars().count() + 1
}

fn span_start(span: Option<SourceSpan>) -> usize {
    span.map_or(usize::MAX, |span| span.byte_start)
}

fn find_readme(repo_root: &Path) -> Option<PathBuf> {
    let candidates = ["README.md", "Readme.md", "readme.md", "README"];
    candidates
        .iter()
        .map(|candidate| repo_root.join(candidate))
        .find(|candidate| candidate.is_file())
}

fn looks_like_badge(alt: &str, target: &str) -> bool {
    let combined = format!("{alt} {target}").to_ascii_lowercase();
    contains_any(
        &combined,
        &[
            "badge",
            "shields.io",
            "github/actions",
            "actions/workflows",
            "ci",
        ],
    )
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn normalize_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
