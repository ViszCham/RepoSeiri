use seiri_core::{
    CoverageIncompleteReason, CoverageStatus, DocumentEvent, DocumentIndex, DocumentRole,
    DocumentRoleCoverage, DocumentScan, DocumentScanInvariantError, DocumentScanStatus, FileKind,
    FileRecord, IndexedDocument, ReadmeSummary, RouteKind, SourceSpan, TextDocumentBase,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentIndexOptions {
    pub max_documents: usize,
    pub max_total_source_bytes: usize,
    pub document: DocumentScanOptions,
}

impl Default for DocumentIndexOptions {
    fn default() -> Self {
        Self {
            max_documents: 32,
            max_total_source_bytes: 4 * 1024 * 1024,
            document: DocumentScanOptions::default(),
        }
    }
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
    let relative_path = normalize_relative_path(repo_root, &readme_path);
    scan_document_file_with_options(&readme_path, relative_path, options).map(Some)
}

pub fn scan_document_index(
    repo_root: impl AsRef<Path>,
    files: &[FileRecord],
    repository_complete: bool,
) -> Result<DocumentIndex, seiri_core::DocumentIndexError> {
    scan_document_index_with_options(
        repo_root,
        files,
        repository_complete,
        &DocumentIndexOptions::default(),
    )
}

pub fn scan_document_index_with_options(
    repo_root: impl AsRef<Path>,
    files: &[FileRecord],
    repository_complete: bool,
    options: &DocumentIndexOptions,
) -> Result<DocumentIndex, seiri_core::DocumentIndexError> {
    let repo_root = repo_root.as_ref();
    let mut candidates = files
        .iter()
        .filter(|record| record.kind == FileKind::File)
        .filter_map(|record| {
            classify_document_role(&record.path)
                .map(|role| (record.path.clone(), role, record.size_bytes))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    candidates.dedup_by(|left, right| left.0 == right.0);

    let mut indexed_documents = 0usize;
    let mut used_source_bytes = 0usize;
    let mut entries = Vec::with_capacity(candidates.len());
    for (path, role, declared_bytes) in candidates {
        if indexed_documents >= options.max_documents {
            entries.push(IndexedDocument::unavailable(
                path,
                role,
                declared_bytes,
                DocumentScanStatus::SkippedDocumentBudget,
            ));
            continue;
        }
        let declared_bytes_usize = usize::try_from(declared_bytes).unwrap_or(usize::MAX);
        if declared_bytes_usize
            > options
                .max_total_source_bytes
                .saturating_sub(used_source_bytes)
        {
            entries.push(IndexedDocument::unavailable(
                path,
                role,
                declared_bytes,
                DocumentScanStatus::SkippedByteBudget,
            ));
            continue;
        }

        let full_path = repo_root.join(&path);
        if !is_markdown_path(&path) {
            match fs::read(&full_path) {
                Ok(bytes) => {
                    used_source_bytes = used_source_bytes.saturating_add(bytes.len());
                    indexed_documents += 1;
                    entries.push(IndexedDocument::unparsed(
                        path,
                        role,
                        declared_bytes,
                        TextDocumentBase::from_bytes(&bytes),
                    ));
                }
                Err(error) => entries.push(IndexedDocument::unavailable(
                    path,
                    role,
                    declared_bytes,
                    if error.kind() == io::ErrorKind::PermissionDenied {
                        DocumentScanStatus::PermissionDenied
                    } else {
                        DocumentScanStatus::ParseFailed
                    },
                )),
            }
            continue;
        }
        match scan_document_file_with_options(&full_path, path.clone(), &options.document) {
            Ok(scan) => {
                used_source_bytes = used_source_bytes.saturating_add(scan.source_bytes());
                indexed_documents += 1;
                entries.push(IndexedDocument::scanned(path, role, declared_bytes, scan));
            }
            Err(error) => entries.push(IndexedDocument::unavailable(
                path,
                role,
                declared_bytes,
                scan_status_for_error(&error),
            )),
        }
    }

    let role_coverage = DocumentRole::ALL
        .into_iter()
        .map(|role| DocumentRoleCoverage {
            role,
            status: role_coverage_status(role, &entries, repository_complete),
        })
        .collect();
    DocumentIndex::try_new(entries, role_coverage)
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

fn scan_document_file_with_options(
    full_path: &Path,
    relative_path: String,
    options: &DocumentScanOptions,
) -> Result<DocumentScan, MarkdownError> {
    let bytes = fs::read(full_path).map_err(|source| MarkdownError::Io {
        path: full_path.to_path_buf(),
        source,
    })?;
    if bytes.len() > options.max_source_bytes {
        return Err(MarkdownError::SourceLimitExceeded {
            path: relative_path,
            bytes: bytes.len(),
            limit: options.max_source_bytes,
        });
    }
    let text = String::from_utf8(bytes).map_err(|error| MarkdownError::InvalidUtf8 {
        path: full_path.to_path_buf(),
        valid_up_to: error.utf8_error().valid_up_to(),
    })?;
    events::scan_text(relative_path, &text, options)
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

#[must_use]
pub fn classify_document_role(path: &str) -> Option<DocumentRole> {
    let normalized = path.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(&lower);

    if is_root_readme_path(&normalized) {
        Some(DocumentRole::RootReadme)
    } else if matches!(file_name, "security.md" | "security") {
        Some(DocumentRole::SecurityPolicy)
    } else if matches!(file_name, "support.md" | "support") {
        Some(DocumentRole::SupportPolicy)
    } else if matches!(file_name, "contributing.md" | "contributing") {
        Some(DocumentRole::ContributionGuide)
    } else if matches!(file_name, "changelog.md" | "changes.md" | "releases.md") {
        Some(DocumentRole::ReleaseNotes)
    } else if matches!(file_name, "governance.md" | "governance") {
        Some(DocumentRole::Governance)
    } else if is_github_configuration_candidate(&lower) {
        Some(DocumentRole::GithubConfiguration)
    } else if lower.starts_with("docs/") && is_markdown_path(&normalized) {
        Some(DocumentRole::Documentation)
    } else if is_markdown_path(&normalized) {
        Some(DocumentRole::OtherMarkdown)
    } else {
        None
    }
}

fn is_github_configuration_candidate(path: &str) -> bool {
    path.starts_with(".github/")
        || matches!(
            path,
            "codeowners" | "renovate.json" | ".renovaterc" | ".renovaterc.json"
        )
}

fn role_coverage_status(
    role: DocumentRole,
    entries: &[IndexedDocument],
    repository_complete: bool,
) -> CoverageStatus {
    if !repository_complete {
        return CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded);
    }
    let matching = entries
        .iter()
        .filter(|entry| entry.role == role)
        .collect::<Vec<_>>();
    if role == DocumentRole::GithubConfiguration && !matching.is_empty() {
        return CoverageStatus::NotRequested;
    }
    matching
        .iter()
        .map(|entry| entry.status.coverage_status())
        .find(|status| *status != CoverageStatus::Complete)
        .unwrap_or(CoverageStatus::Complete)
}

fn scan_status_for_error(error: &MarkdownError) -> DocumentScanStatus {
    match error {
        MarkdownError::InvalidUtf8 { .. } => DocumentScanStatus::InvalidUtf8,
        MarkdownError::Io { source, .. } if source.kind() == io::ErrorKind::PermissionDenied => {
            DocumentScanStatus::PermissionDenied
        }
        MarkdownError::Io { .. }
        | MarkdownError::SourceLimitExceeded { .. }
        | MarkdownError::EventLimitExceeded { .. }
        | MarkdownError::DiagnosticLimitExceeded { .. }
        | MarkdownError::Invariant(_) => DocumentScanStatus::ParseFailed,
    }
}

fn is_root_readme_path(path: &str) -> bool {
    matches!(path, "README.md" | "Readme.md" | "readme.md" | "README")
}

fn is_markdown_path(path: &str) -> bool {
    is_root_readme_path(path) || path.to_ascii_lowercase().ends_with(".md")
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
