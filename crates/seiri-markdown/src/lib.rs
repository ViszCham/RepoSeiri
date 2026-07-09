use seiri_core::{
    MarkdownBadge, MarkdownHeading, MarkdownLink, ReadmeSummary, RouteCandidate, RouteKind,
    RouteSource,
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

    for (zero_index, line) in text.lines().enumerate() {
        let line_number = zero_index + 1;
        if let Some(heading) = parse_heading(line, line_number) {
            let route = classify_route(&heading.text, None);
            if route != RouteKind::Unknown {
                route_candidates.push(RouteCandidate {
                    route,
                    source: RouteSource::Heading,
                    text: heading.text.clone(),
                    target: None,
                    line: line_number,
                });
            }
            headings.push(heading);
        }

        for image in parse_markdown_links(line, line_number, true) {
            if looks_like_badge(&image.text, &image.target) {
                route_candidates.push(RouteCandidate {
                    route: RouteKind::Automation,
                    source: RouteSource::Badge,
                    text: image.text.clone(),
                    target: Some(image.target.clone()),
                    line: line_number,
                });
                badges.push(MarkdownBadge {
                    alt: image.text,
                    target: image.target,
                    line: line_number,
                });
            }
        }

        for link in parse_markdown_links(line, line_number, false) {
            let route = classify_route(&link.text, Some(&link.target));
            if route != RouteKind::Unknown {
                route_candidates.push(RouteCandidate {
                    route,
                    source: RouteSource::Link,
                    text: link.text.clone(),
                    target: Some(link.target.clone()),
                    line: line_number,
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

fn parse_heading(line: &str, line_number: usize) -> Option<MarkdownHeading> {
    let trimmed = line.trim_start();
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
        line: line_number,
    })
}

fn parse_markdown_links(line: &str, line_number: usize, images_only: bool) -> Vec<MarkdownLink> {
    let bytes = line.as_bytes();
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
        let Some(label_end_offset) = line[label_start..].find(']') else {
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
        let Some(target_end_offset) = line[target_start..].find(')') else {
            cursor = target_start;
            continue;
        };
        let target_end = target_start + target_end_offset;
        let text = line[label_start..label_end].trim();
        let target = line[target_start..target_end].trim();
        if !text.is_empty() && !target.is_empty() {
            out.push(MarkdownLink {
                text: text.to_string(),
                target: target.to_string(),
                line: line_number,
                route: None,
            });
        }
        cursor = target_end + 1;
    }

    out
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
