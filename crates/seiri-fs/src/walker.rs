use crate::{RepoPathError, RepoRelativePath};
use seiri_core::{FileKind, FileRecord, IgnoredPathReason, IgnoredShallowRecord};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DEFAULT_IGNORED_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    ".venv",
    "dist",
    "build",
    ".idea",
    ".vscode",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    pub max_depth: usize,
    pub max_entries: usize,
    pub max_directory_entries: usize,
    pub ignore_policy: IgnorePolicy,
    pub max_ignored_records: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 32,
            max_entries: 100_000,
            max_directory_entries: 16_384,
            ignore_policy: IgnorePolicy::default(),
            max_ignored_records: 4_096,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnorePolicy {
    additional_names: Vec<String>,
}

impl IgnorePolicy {
    #[must_use]
    pub fn with_additional_names(mut additional_names: Vec<String>) -> Self {
        additional_names.sort();
        additional_names.dedup();
        Self { additional_names }
    }

    #[must_use]
    pub fn additional_names(&self) -> &[String] {
        &self.additional_names
    }

    fn ignores(&self, name: &str) -> bool {
        DEFAULT_IGNORED_NAMES.contains(&name)
            || self
                .additional_names
                .binary_search_by(|candidate| candidate.as_str().cmp(name))
                .is_ok()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RepositoryRoot(PathBuf);

impl std::fmt::Debug for RepositoryRoot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("RepositoryRoot").field(&".").finish()
    }
}

impl RepositoryRoot {
    pub fn resolve(path: &Path) -> Result<Self, FsError> {
        let canonical = fs::canonicalize(path).map_err(|source| FsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if !canonical.is_dir() {
            return Err(FsError::NotDirectory(canonical));
        }
        if has_repo_boundary_marker(&canonical) {
            return Ok(Self(canonical));
        }

        let mut cursor = Some(canonical.as_path());
        while let Some(candidate) = cursor {
            if candidate.join(".git").exists() {
                return Ok(Self(candidate.to_path_buf()));
            }
            cursor = candidate.parent();
        }
        Ok(Self(canonical))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    #[must_use]
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkLimitKind {
    Depth,
    Entries,
    DirectoryEntries,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkTruncation {
    pub kind: WalkLimitKind,
    pub path: String,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkCompletion {
    Complete,
    Truncated(WalkTruncation),
}

impl WalkCompletion {
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWalkSummary {
    pub max_depth: usize,
    pub max_entries: usize,
    pub max_directory_entries: usize,
    pub visited_entries: usize,
    pub ignored_entries: usize,
    pub ignored_records_truncated: bool,
    pub max_depth_reached: usize,
    pub completion: WalkCompletion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWalk {
    root: RepositoryRoot,
    records: Vec<FileRecord>,
    summary: RepositoryWalkSummary,
    ignored_shallow: Vec<IgnoredShallowRecord>,
}

impl RepositoryWalk {
    #[must_use]
    pub fn root(&self) -> &RepositoryRoot {
        &self.root
    }

    #[must_use]
    pub fn records(&self) -> &[FileRecord] {
        &self.records
    }

    #[must_use]
    pub const fn summary(&self) -> &RepositoryWalkSummary {
        &self.summary
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        RepositoryRoot,
        Vec<FileRecord>,
        RepositoryWalkSummary,
        Vec<IgnoredShallowRecord>,
    ) {
        (self.root, self.records, self.summary, self.ignored_shallow)
    }
}

pub enum FsError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    NotDirectory(PathBuf),
    LimitExceeded {
        kind: WalkLimitKind,
        path: String,
        limit: usize,
    },
    InvalidRepositoryPath(RepoPathError),
}

impl std::fmt::Debug for FsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl Display for FsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { source, .. } => {
                write!(formatter, "failed to read repository path: {source}")
            }
            Self::NotDirectory(_) => formatter.write_str("repository path is not a directory"),
            Self::LimitExceeded { kind, path, limit } => write!(
                formatter,
                "repository walk {kind:?} limit {limit} exceeded at {path}"
            ),
            Self::InvalidRepositoryPath(kind) => {
                write!(
                    formatter,
                    "repository path cannot be represented safely: {kind:?}"
                )
            }
        }
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NotDirectory(_) | Self::LimitExceeded { .. } | Self::InvalidRepositoryPath(_) => {
                None
            }
        }
    }
}

pub fn walk_repository(path: impl AsRef<Path>) -> Result<RepositoryWalk, FsError> {
    walk_repository_with_options(path, &ScanOptions::default())
}

pub fn walk_repository_with_options(
    path: impl AsRef<Path>,
    options: &ScanOptions,
) -> Result<RepositoryWalk, FsError> {
    let root = RepositoryRoot::resolve(path.as_ref())?;
    let mut state = WalkState::default();
    walk_dir(root.as_path(), root.as_path(), 0, options, &mut state)?;
    state
        .records
        .sort_by(|left, right| left.path.cmp(&right.path));
    let summary = RepositoryWalkSummary {
        max_depth: options.max_depth,
        max_entries: options.max_entries,
        max_directory_entries: options.max_directory_entries,
        visited_entries: state.records.len(),
        ignored_entries: state.ignored_entries,
        ignored_records_truncated: state.ignored_records_truncated,
        max_depth_reached: state.max_depth_reached,
        completion: state.completion.unwrap_or(WalkCompletion::Complete),
    };
    Ok(RepositoryWalk {
        root,
        records: state.records,
        summary,
        ignored_shallow: state.ignored_shallow,
    })
}

pub fn resolve_repo_root(path: &Path) -> Result<PathBuf, FsError> {
    RepositoryRoot::resolve(path).map(RepositoryRoot::into_path_buf)
}

#[derive(Default)]
struct WalkState {
    records: Vec<FileRecord>,
    ignored_entries: usize,
    ignored_shallow: Vec<IgnoredShallowRecord>,
    ignored_records_truncated: bool,
    max_depth_reached: usize,
    completion: Option<WalkCompletion>,
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    options: &ScanOptions,
    state: &mut WalkState,
) -> Result<(), FsError> {
    if state.completion.is_some() {
        return Ok(());
    }

    let mut entries = Vec::with_capacity(options.max_directory_entries.min(1_024));
    for entry in fs::read_dir(dir).map_err(|source| FsError::Io {
        path: dir.to_path_buf(),
        source,
    })? {
        if entries.len() >= options.max_directory_entries {
            state.completion = Some(WalkCompletion::Truncated(WalkTruncation {
                kind: WalkLimitKind::DirectoryEntries,
                path: walk_display_path(root, dir)?,
                limit: options.max_directory_entries,
            }));
            return Ok(());
        }
        entries.push(entry.map_err(|source| FsError::Io {
            path: dir.to_path_buf(),
            source,
        })?);
    }
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or(FsError::InvalidRepositoryPath(RepoPathError::NonUtf8))?;
        let file_type = entry.file_type().map_err(|source| FsError::Io {
            path: path.clone(),
            source,
        })?;
        if let Some(reason) = ignore_reason(name, options) {
            state.ignored_entries += 1;
            if state.ignored_shallow.len() < options.max_ignored_records {
                state.ignored_shallow.push(IgnoredShallowRecord {
                    path: normalize_relative_path(root, &path)?,
                    kind: file_kind(file_type),
                    reason,
                });
            } else {
                state.ignored_records_truncated = true;
            }
            continue;
        }
        if state.records.len() >= options.max_entries {
            state.completion = Some(WalkCompletion::Truncated(WalkTruncation {
                kind: WalkLimitKind::Entries,
                path: normalize_relative_path(root, &path)?,
                limit: options.max_entries,
            }));
            return Ok(());
        }

        let kind = file_kind(file_type);
        let size_bytes = if matches!(kind, FileKind::File) {
            entry
                .metadata()
                .map_err(|source| FsError::Io {
                    path: path.clone(),
                    source,
                })?
                .len()
        } else {
            0
        };

        state.max_depth_reached = state.max_depth_reached.max(depth);
        state.records.push(FileRecord {
            path: normalize_relative_path(root, &path)?,
            kind,
            size_bytes,
        });

        if file_type.is_dir() {
            if depth >= options.max_depth {
                state.completion = Some(WalkCompletion::Truncated(WalkTruncation {
                    kind: WalkLimitKind::Depth,
                    path: normalize_relative_path(root, &path)?,
                    limit: options.max_depth,
                }));
                return Ok(());
            }
            walk_dir(root, &path, depth + 1, options, state)?;
        }
    }
    Ok(())
}

fn ignore_reason(name: &str, options: &ScanOptions) -> Option<IgnoredPathReason> {
    if !options.ignore_policy.ignores(name) {
        return None;
    }
    Some(match name {
        ".git" | ".hg" | ".svn" => IgnoredPathReason::GitMetadata,
        "target" | "build" => IgnoredPathReason::BuildOutput,
        "node_modules" => IgnoredPathReason::DependencyTree,
        ".venv" => IgnoredPathReason::VirtualEnvironment,
        ".idea" | ".vscode" => IgnoredPathReason::EditorState,
        "dist" => IgnoredPathReason::DistributionOutput,
        _ => IgnoredPathReason::UserConfigured,
    })
}

fn file_kind(file_type: fs::FileType) -> FileKind {
    if file_type.is_symlink() {
        FileKind::Symlink
    } else if file_type.is_dir() {
        FileKind::Directory
    } else {
        FileKind::File
    }
}

fn has_repo_boundary_marker(path: &Path) -> bool {
    [
        "README.md",
        "Readme.md",
        "readme.md",
        "README",
        "LICENSE",
        "Cargo.toml",
    ]
    .iter()
    .any(|marker| path.join(marker).exists())
        || path.join("docs").is_dir()
        || path.join(".github").is_dir()
}

fn normalize_relative_path(root: &Path, path: &Path) -> Result<String, FsError> {
    RepoRelativePath::from_rooted(root, path)
        .map(RepoRelativePath::into_string)
        .map_err(FsError::InvalidRepositoryPath)
}

fn walk_display_path(root: &Path, path: &Path) -> Result<String, FsError> {
    if root == path {
        Ok(".".to_string())
    } else {
        normalize_relative_path(root, path)
    }
}
