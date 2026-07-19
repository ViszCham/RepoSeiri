#![forbid(unsafe_code)]

use seiri_core::{
    CoverageIncompleteReason, CoverageStatus, DocumentEvent, DocumentIndex, DocumentRole,
    DocumentRoleCoverage, DocumentScan, DocumentScanInvariantError, DocumentScanStatus, FileKind,
    FileRecord, IndexedDocument, PathClassification, ReadmeSummary, RepositoryScopeGraph,
    SourceDocument, SourceSpan, SourceStoreError, TextDocumentBase,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};

mod classifier;
mod context;
mod events;
mod route_map;
mod source;

use route_map::build_route_map;
use source::{
    prepare_markdown_document, read_bounded_file, scan_status_for_error, PreparedMarkdown,
};

pub use classifier::{classify_route, classify_routes};
pub use source::DocumentSourceSession;

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
            max_documents: 256,
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
    #[must_use]
    pub fn derived_for_source(source_bytes: usize) -> Self {
        Self {
            max_source_bytes: source_bytes,
            max_events: source_bytes.saturating_mul(2).saturating_add(1),
            max_diagnostics: source_bytes.saturating_add(1),
        }
    }
}

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

impl std::fmt::Debug for MarkdownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl Display for MarkdownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { source, .. } => {
                write!(formatter, "failed to read repository markdown: {source}")
            }
            Self::InvalidUtf8 { valid_up_to, .. } => write!(
                formatter,
                "repository markdown is not valid UTF-8 after byte {valid_up_to}"
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
    scan_document_index_with_options_and_scope(repo_root, files, repository_complete, options, None)
}

pub fn scan_document_index_with_options_and_scope(
    repo_root: impl AsRef<Path>,
    files: &[FileRecord],
    repository_complete: bool,
    options: &DocumentIndexOptions,
    scope_graph: Option<&RepositoryScopeGraph>,
) -> Result<DocumentIndex, seiri_core::DocumentIndexError> {
    scan_document_source_session_with_options_and_scope(
        repo_root,
        files,
        repository_complete,
        options,
        scope_graph,
    )
    .map(DocumentSourceSession::into_index)
}

pub fn scan_document_source_session_with_options_and_scope(
    repo_root: impl AsRef<Path>,
    files: &[FileRecord],
    repository_complete: bool,
    options: &DocumentIndexOptions,
    scope_graph: Option<&RepositoryScopeGraph>,
) -> Result<DocumentSourceSession, seiri_core::DocumentIndexError> {
    let repo_root = repo_root.as_ref();
    let mut candidates = files
        .iter()
        .filter(|record| record.kind == FileKind::File)
        .filter_map(|record| {
            classify_document_role(&record.path).map(|role| DocumentCandidate {
                path: record.path.clone(),
                role,
                classification: PathClassification::classify(&record.path, scope_graph),
                declared_bytes: record.size_bytes,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    candidates.dedup_by(|left, right| left.path == right.path);

    let mut prepared = BTreeMap::new();
    let readme_links = readme_linked_targets(repo_root, &candidates, options, &mut prepared);
    candidates.sort_by(|left, right| {
        selection_rank(left, &readme_links)
            .cmp(&selection_rank(right, &readme_links))
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut indexed_documents = 0usize;
    let mut used_source_bytes = 0usize;
    let mut entries = Vec::with_capacity(candidates.len());
    let mut sources = Vec::new();
    for candidate in candidates {
        let DocumentCandidate {
            path,
            role,
            classification,
            declared_bytes,
        } = candidate;
        if indexed_documents >= options.max_documents {
            entries.push(IndexedDocument::unavailable(
                path,
                role,
                classification,
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
                classification,
                declared_bytes,
                DocumentScanStatus::SkippedByteBudget,
            ));
            continue;
        }

        let full_path = repo_root.join(&path);
        if !is_markdown_path(&path) {
            let remaining = options
                .max_total_source_bytes
                .saturating_sub(used_source_bytes);
            match read_bounded_file(&full_path, &path, remaining) {
                Ok(bytes) => {
                    used_source_bytes = used_source_bytes.saturating_add(bytes.len());
                    indexed_documents += 1;
                    let base = TextDocumentBase::from_bytes(&bytes);
                    sources.push(SourceDocument::from_bytes(path.clone(), bytes));
                    entries.push(IndexedDocument::unparsed(
                        path,
                        role,
                        classification,
                        declared_bytes,
                        base,
                    ));
                }
                Err(error) => entries.push(IndexedDocument::unavailable(
                    path,
                    role,
                    classification,
                    declared_bytes,
                    scan_status_for_error(&error),
                )),
            }
            continue;
        }
        let remaining = options
            .max_total_source_bytes
            .saturating_sub(used_source_bytes);
        let mut document_options = options.document.clone();
        document_options.max_source_bytes = document_options.max_source_bytes.min(remaining);
        let prepared_document = prepared
            .remove(&path)
            .unwrap_or_else(|| prepare_markdown_document(&full_path, &path, &document_options));
        match prepared_document {
            PreparedMarkdown::Scanned { scan, source } => {
                used_source_bytes = used_source_bytes.saturating_add(scan.source_bytes());
                indexed_documents += 1;
                sources.push(SourceDocument::from_text(path.clone(), source));
                entries.push(IndexedDocument::scanned(
                    path,
                    role,
                    classification,
                    declared_bytes,
                    scan,
                ));
            }
            PreparedMarkdown::Unavailable(status) => entries.push(IndexedDocument::unavailable(
                path,
                role,
                classification,
                declared_bytes,
                status,
            )),
        }
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    let role_coverage = DocumentRole::ALL
        .into_iter()
        .map(|role| DocumentRoleCoverage {
            role,
            status: role_coverage_status(role, &entries, repository_complete),
        })
        .collect();
    let index = DocumentIndex::try_new(entries, role_coverage)?;
    sources.sort_by(|left, right| left.path().cmp(right.path()));
    DocumentSourceSession::new(index, sources).map_err(map_source_store_error)
}

fn map_source_store_error(error: SourceStoreError) -> seiri_core::DocumentIndexError {
    match error {
        SourceStoreError::EmptyPath => seiri_core::DocumentIndexError::EmptyPath,
        SourceStoreError::DuplicatePath(path) => {
            seiri_core::DocumentIndexError::DuplicatePath(path)
        }
        SourceStoreError::NonCanonicalOrder | SourceStoreError::TotalBytesOverflow => {
            seiri_core::DocumentIndexError::NonCanonicalEntryOrder
        }
    }
}

#[derive(Debug)]
struct DocumentCandidate {
    path: String,
    role: DocumentRole,
    classification: PathClassification,
    declared_bytes: u64,
}

fn readme_linked_targets(
    repo_root: &Path,
    candidates: &[DocumentCandidate],
    options: &DocumentIndexOptions,
    prepared: &mut BTreeMap<String, PreparedMarkdown>,
) -> BTreeSet<String> {
    let Some(readme) = candidates
        .iter()
        .find(|candidate| candidate.role == DocumentRole::RootReadme)
    else {
        return BTreeSet::new();
    };
    if options.max_documents == 0
        || usize::try_from(readme.declared_bytes).unwrap_or(usize::MAX)
            > options.max_total_source_bytes
    {
        return BTreeSet::new();
    }
    let prepared_readme = prepare_markdown_document(
        &repo_root.join(&readme.path),
        &readme.path,
        &options.document,
    );
    let PreparedMarkdown::Scanned { scan, .. } = &prepared_readme else {
        prepared.insert(readme.path.clone(), prepared_readme);
        return BTreeSet::new();
    };
    let targets = scan
        .events()
        .iter()
        .filter_map(|event| match event {
            DocumentEvent::Link(link) => local_target_key(&link.target),
            _ => None,
        })
        .collect();
    prepared.insert(readme.path.clone(), prepared_readme);
    targets
}

fn local_target_key(target: &str) -> Option<String> {
    let trimmed = target.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
    {
        return None;
    }
    let path = trimmed
        .split(['#', '?'])
        .next()
        .unwrap_or(trimmed)
        .trim_start_matches("./")
        .replace('\\', "/");
    (!path.is_empty()).then(|| path.to_ascii_lowercase())
}

fn selection_rank(candidate: &DocumentCandidate, readme_links: &BTreeSet<String>) -> (u8, u8) {
    let normalized = candidate.path.replace('\\', "/").to_ascii_lowercase();
    if candidate.role == DocumentRole::RootReadme {
        return (0, 0);
    }
    if !normalized.contains('/')
        && matches!(
            candidate.role,
            DocumentRole::SecurityPolicy
                | DocumentRole::SupportPolicy
                | DocumentRole::ContributionGuide
                | DocumentRole::ReleaseNotes
                | DocumentRole::Governance
        )
    {
        return (1, candidate.role as u8);
    }
    if readme_links.contains(&normalized) {
        return (2, candidate.role as u8);
    }
    if matches!(normalized.as_str(), "docs/readme.md" | "docs/index.md") {
        return (3, candidate.role as u8);
    }
    if candidate.classification.is_primary_repository_content() {
        (4, candidate.role as u8)
    } else {
        (5, candidate.role as u8)
    }
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
    let bytes = read_bounded_file(full_path, &relative_path, options.max_source_bytes)?;
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
    let document = scan_document_with_options(
        path,
        text,
        &DocumentScanOptions::derived_for_source(text.len()),
    )
    .expect("in-memory limits are derived from the supplied source");
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
            DocumentEvent::VisibleProse(_) => {}
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
            "codeowners" | "docs/codeowners" | "renovate.json" | ".renovaterc" | ".renovaterc.json"
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
        .filter(|entry| entry.role == role && entry.classification.is_primary_repository_content())
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
    seiri_fs::RepoRelativePath::from_rooted(root, path).map_or_else(
        |_| "<invalid-repository-path>".to_string(),
        seiri_fs::RepoRelativePath::into_string,
    )
}

fn span_start(span: Option<SourceSpan>) -> usize {
    span.map_or(usize::MAX, |span| span.byte_start)
}
