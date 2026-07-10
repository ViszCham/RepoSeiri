use seiri_core::{
    DocumentEvent, DocumentScan, DocumentScanInvariantError, ReadmeSummary, RouteKind, SourceSpan,
};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod events;
mod route_map;

use route_map::build_route_map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentScanOptions {
    pub max_source_bytes: usize,
    pub max_events: usize,
    pub max_diagnostics: usize,
}

impl Default for DocumentScanOptions {
    fn default() -> Self {
        Self {
            max_source_bytes: 2 * 1024 * 1024,
            max_events: 65_536,
            max_diagnostics: 1_024,
        }
    }
}

impl DocumentScanOptions {
    fn compatibility(source_bytes: usize) -> Self {
        Self {
            max_source_bytes: source_bytes,
            max_events: source_bytes.saturating_mul(2).saturating_add(1),
            max_diagnostics: source_bytes.saturating_add(1),
        }
    }
}

#[derive(Debug)]
pub enum MarkdownError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    InvalidUtf8 {
        path: PathBuf,
        valid_up_to: usize,
    },
    SourceLimitExceeded {
        path: String,
        bytes: usize,
        limit: usize,
    },
    EventLimitExceeded {
        path: String,
        limit: usize,
    },
    DiagnosticLimitExceeded {
        path: String,
        limit: usize,
    },
    Invariant(DocumentScanInvariantError),
}

impl Display for MarkdownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "failed to read markdown {}: {source}",
                    path.display()
                )
            }
            Self::InvalidUtf8 { path, valid_up_to } => write!(
                formatter,
                "markdown {} is not valid UTF-8 after byte {valid_up_to}",
                path.display()
            ),
            Self::SourceLimitExceeded { path, bytes, limit } => write!(
                formatter,
                "markdown {path} has {bytes} bytes and exceeds source limit {limit}"
            ),
            Self::EventLimitExceeded { path, limit } => {
                write!(formatter, "markdown {path} exceeds event limit {limit}")
            }
            Self::DiagnosticLimitExceeded { path, limit } => {
                write!(
                    formatter,
                    "markdown {path} exceeds diagnostic limit {limit}"
                )
            }
            Self::Invariant(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for MarkdownError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Invariant(error) => Some(error),
            Self::InvalidUtf8 { .. }
            | Self::SourceLimitExceeded { .. }
            | Self::EventLimitExceeded { .. }
            | Self::DiagnosticLimitExceeded { .. } => None,
        }
    }
}

pub fn scan_readme_document(
    repo_root: impl AsRef<Path>,
) -> Result<Option<DocumentScan>, MarkdownError> {
    scan_readme_document_with_options(repo_root, &DocumentScanOptions::default())
}

pub fn scan_readme_document_with_options(
    repo_root: impl AsRef<Path>,
    options: &DocumentScanOptions,
) -> Result<Option<DocumentScan>, MarkdownError> {
    let repo_root = repo_root.as_ref();
    let Some(readme_path) = find_readme(repo_root) else {
        return Ok(None);
    };
    let bytes = fs::read(&readme_path).map_err(|source| MarkdownError::Io {
        path: readme_path.clone(),
        source,
    })?;
    let relative_path = normalize_relative_path(repo_root, &readme_path);
    if bytes.len() > options.max_source_bytes {
        return Err(MarkdownError::SourceLimitExceeded {
            path: relative_path,
            bytes: bytes.len(),
            limit: options.max_source_bytes,
        });
    }
    let text = String::from_utf8(bytes).map_err(|error| MarkdownError::InvalidUtf8 {
        path: readme_path,
        valid_up_to: error.utf8_error().valid_up_to(),
    })?;
    events::scan_text(relative_path, &text, options).map(Some)
}

pub fn scan_document(path: impl Into<String>, text: &str) -> Result<DocumentScan, MarkdownError> {
    scan_document_with_options(path, text, &DocumentScanOptions::default())
}

pub fn scan_document_with_options(
    path: impl Into<String>,
    text: &str,
    options: &DocumentScanOptions,
) -> Result<DocumentScan, MarkdownError> {
    events::scan_text(path.into(), text, options)
}

pub fn analyze_readme(repo_root: impl AsRef<Path>) -> Result<Option<ReadmeSummary>, MarkdownError> {
    let repo_root = repo_root.as_ref();
    scan_readme_document(repo_root).map(|document| {
        document.map(|document| summarize_readme_document(&document, Some(repo_root)))
    })
}

pub fn parse_readme(path: impl Into<String>, text: &str) -> ReadmeSummary {
    let document =
        scan_document_with_options(path, text, &DocumentScanOptions::compatibility(text.len()))
            .expect("in-memory compatibility limits are derived from the supplied source");
    summarize_readme_document(&document, None)
}

#[must_use]
pub fn summarize_readme_document(
    document: &DocumentScan,
    repo_root: Option<&Path>,
) -> ReadmeSummary {
    let mut headings = Vec::new();
    let mut links = Vec::new();
    let mut badges = Vec::new();
    let mut route_candidates = Vec::new();

    for event in document.events() {
        match event {
            DocumentEvent::Heading(value) => headings.push(value.clone()),
            DocumentEvent::Link(value) => links.push(value.clone()),
            DocumentEvent::Badge(value) => badges.push(value.clone()),
            DocumentEvent::RouteCandidate(value) => route_candidates.push(value.clone()),
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
        path: document.path().to_string(),
        route_map: build_route_map(&route_candidates, repo_root),
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

fn span_start(span: Option<SourceSpan>) -> usize {
    span.map_or(usize::MAX, |span| span.byte_start)
}
