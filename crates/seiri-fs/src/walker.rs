use seiri_core::{FileKind, FileRecord};
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
    pub ignore_policy: IgnorePolicy,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 32,
            max_entries: 100_000,
            ignore_policy: IgnorePolicy::default(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRoot(PathBuf);

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWalkSummary {
    pub max_depth: usize,
    pub max_entries: usize,
    pub visited_entries: usize,
    pub ignored_entries: usize,
    pub max_depth_reached: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryWalk {
    root: RepositoryRoot,
    records: Vec<FileRecord>,
    summary: RepositoryWalkSummary,
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

    pub(crate) fn into_parts(self) -> (RepositoryRoot, Vec<FileRecord>, RepositoryWalkSummary) {
        (self.root, self.records, self.summary)
    }
}

#[derive(Debug)]
pub enum FsError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    NotDirectory(PathBuf),
    LimitExceeded {
        kind: WalkLimitKind,
        path: PathBuf,
        limit: usize,
    },
}

impl Display for FsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::NotDirectory(path) => write!(formatter, "{} is not a directory", path.display()),
            Self::LimitExceeded { kind, path, limit } => write!(
                formatter,
                "repository walk {kind:?} limit {limit} exceeded at {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NotDirectory(_) | Self::LimitExceeded { .. } => None,
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
        visited_entries: state.records.len(),
        ignored_entries: state.ignored_entries,
        max_depth_reached: state.max_depth_reached,
    };
    Ok(RepositoryWalk {
        root,
        records: state.records,
        summary,
    })
}

pub fn resolve_repo_root(path: &Path) -> Result<PathBuf, FsError> {
    RepositoryRoot::resolve(path).map(RepositoryRoot::into_path_buf)
}

#[derive(Default)]
struct WalkState {
    records: Vec<FileRecord>,
    ignored_entries: usize,
    max_depth_reached: usize,
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    options: &ScanOptions,
    state: &mut WalkState,
) -> Result<(), FsError> {
    let entries = fs::read_dir(dir).map_err(|source| FsError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry_result in entries {
        let entry = entry_result.map_err(|source| FsError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        if should_ignore_name(&name.to_string_lossy(), options) {
            state.ignored_entries += 1;
            continue;
        }
        if state.records.len() >= options.max_entries {
            return Err(FsError::LimitExceeded {
                kind: WalkLimitKind::Entries,
                path,
                limit: options.max_entries,
            });
        }

        let file_type = entry.file_type().map_err(|source| FsError::Io {
            path: path.clone(),
            source,
        })?;
        let kind = if file_type.is_symlink() {
            FileKind::Symlink
        } else if file_type.is_dir() {
            FileKind::Directory
        } else {
            FileKind::File
        };
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
            path: normalize_relative_path(root, &path),
            kind,
            size_bytes,
        });

        if file_type.is_dir() {
            if depth >= options.max_depth {
                return Err(FsError::LimitExceeded {
                    kind: WalkLimitKind::Depth,
                    path,
                    limit: options.max_depth,
                });
            }
            walk_dir(root, &path, depth + 1, options, state)?;
        }
    }
    Ok(())
}

fn should_ignore_name(name: &str, options: &ScanOptions) -> bool {
    options.ignore_policy.ignores(name)
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

fn normalize_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
