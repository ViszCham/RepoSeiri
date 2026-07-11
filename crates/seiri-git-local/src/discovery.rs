use seiri_core::{
    AnalysisScope, GitDiagnostic, GitDiagnosticKind, RepositoryRootKind, RepositoryScopeRoot,
};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const MAX_GITFILE_BYTES: u64 = 4_096;

#[derive(Debug, Clone)]
pub struct DiscoveredRepository {
    root: RepositoryScopeRoot,
    analysis_root: PathBuf,
    worktree_root: Option<PathBuf>,
    git_dir: Option<PathBuf>,
    common_dir: Option<PathBuf>,
    diagnostic: Option<GitDiagnostic>,
}

impl DiscoveredRepository {
    #[must_use]
    pub fn root(&self) -> &RepositoryScopeRoot {
        &self.root
    }

    #[must_use]
    pub fn analysis_root(&self) -> &Path {
        &self.analysis_root
    }

    #[must_use]
    pub fn worktree_root(&self) -> Option<&Path> {
        self.worktree_root.as_deref()
    }

    #[must_use]
    pub fn git_dir(&self) -> Option<&Path> {
        self.git_dir.as_deref()
    }

    #[must_use]
    pub fn common_dir(&self) -> Option<&Path> {
        self.common_dir.as_deref()
    }

    #[must_use]
    pub fn diagnostic(&self) -> Option<&GitDiagnostic> {
        self.diagnostic.as_ref()
    }
}

#[derive(Debug)]
pub enum RepositoryDiscoveryError {
    Io { path: PathBuf, source: io::Error },
    NotDirectory(PathBuf),
    GitFileTooLarge(PathBuf),
    MalformedGitFile(PathBuf),
    InvalidGitDirectory(PathBuf),
}

impl Display for RepositoryDiscoveryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::NotDirectory(path) => write!(formatter, "{} is not a directory", path.display()),
            Self::GitFileTooLarge(path) => {
                write!(formatter, "gitfile {} exceeds 4096 bytes", path.display())
            }
            Self::MalformedGitFile(path) => {
                write!(formatter, "gitfile {} is malformed", path.display())
            }
            Self::InvalidGitDirectory(path) => write!(
                formatter,
                "git metadata directory {} is invalid",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RepositoryDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NotDirectory(_)
            | Self::GitFileTooLarge(_)
            | Self::MalformedGitFile(_)
            | Self::InvalidGitDirectory(_) => None,
        }
    }
}

pub fn discover_repository(
    input: &Path,
    scope: AnalysisScope,
) -> Result<DiscoveredRepository, RepositoryDiscoveryError> {
    let canonical = canonical_directory(input)?;
    if is_bare_repository(&canonical) {
        return Ok(build(
            canonical.clone(),
            None,
            Some(canonical.clone()),
            Some(canonical),
            RepositoryRootKind::Bare,
            scope,
            None,
        ));
    }
    let Some((worktree_root, marker)) = nearest_git_marker(&canonical) else {
        return Ok(no_git(canonical, scope));
    };
    let (git_dir, common_dir, linked) = match resolve_git_marker(&marker) {
        Ok(value) => value,
        Err(error) => {
            let analysis_root = if scope == AnalysisScope::Subtree {
                canonical
            } else {
                worktree_root.clone()
            };
            return Ok(build(
                analysis_root,
                Some(worktree_root),
                None,
                None,
                RepositoryRootKind::MalformedGit,
                scope,
                Some(discovery_diagnostic(&error)),
            ));
        }
    };
    let analysis_root = match scope {
        AnalysisScope::Repository => worktree_root.clone(),
        AnalysisScope::Workspace => nearest_workspace_root(&canonical, &worktree_root),
        AnalysisScope::Subtree => canonical.clone(),
    };
    let kind = if scope == AnalysisScope::Subtree && analysis_root != worktree_root {
        RepositoryRootKind::Subtree
    } else if linked {
        RepositoryRootKind::LinkedWorktree
    } else {
        RepositoryRootKind::Worktree
    };
    Ok(build(
        analysis_root,
        Some(worktree_root),
        Some(git_dir),
        Some(common_dir),
        kind,
        scope,
        None,
    ))
}

fn canonical_directory(input: &Path) -> Result<PathBuf, RepositoryDiscoveryError> {
    let canonical = fs::canonicalize(input).map_err(|source| RepositoryDiscoveryError::Io {
        path: input.to_path_buf(),
        source,
    })?;
    let directory = if canonical.is_file() {
        canonical
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| RepositoryDiscoveryError::NotDirectory(canonical.clone()))?
    } else {
        canonical
    };
    if !directory.is_dir() {
        return Err(RepositoryDiscoveryError::NotDirectory(directory));
    }
    Ok(directory)
}

fn nearest_git_marker(start: &Path) -> Option<(PathBuf, PathBuf)> {
    start.ancestors().find_map(|candidate| {
        let marker = candidate.join(".git");
        marker.exists().then(|| (candidate.to_path_buf(), marker))
    })
}

fn resolve_git_marker(marker: &Path) -> Result<(PathBuf, PathBuf, bool), RepositoryDiscoveryError> {
    let linked = marker.is_file();
    let git_dir = if marker.is_dir() {
        fs::canonicalize(marker).map_err(|source| RepositoryDiscoveryError::Io {
            path: marker.to_path_buf(),
            source,
        })?
    } else {
        let body = read_bounded_utf8(marker)?;
        let value = body
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("gitdir:"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| RepositoryDiscoveryError::MalformedGitFile(marker.to_path_buf()))?;
        let unresolved = marker
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(value);
        fs::canonicalize(&unresolved).map_err(|source| RepositoryDiscoveryError::Io {
            path: unresolved,
            source,
        })?
    };
    if !git_dir.is_dir() || !git_dir.join("HEAD").is_file() {
        return Err(RepositoryDiscoveryError::InvalidGitDirectory(git_dir));
    }
    let common_dir = resolve_common_dir(&git_dir)?;
    Ok((git_dir, common_dir, linked))
}

fn resolve_common_dir(git_dir: &Path) -> Result<PathBuf, RepositoryDiscoveryError> {
    let marker = git_dir.join("commondir");
    if !marker.is_file() {
        return Ok(git_dir.to_path_buf());
    }
    let value = read_bounded_utf8(&marker)?.trim().to_string();
    if value.is_empty() {
        return Err(RepositoryDiscoveryError::MalformedGitFile(marker));
    }
    let unresolved = git_dir.join(value);
    let canonical =
        fs::canonicalize(&unresolved).map_err(|source| RepositoryDiscoveryError::Io {
            path: unresolved,
            source,
        })?;
    if !canonical.is_dir() {
        return Err(RepositoryDiscoveryError::InvalidGitDirectory(canonical));
    }
    Ok(canonical)
}

fn read_bounded_utf8(path: &Path) -> Result<String, RepositoryDiscoveryError> {
    let handle = fs::File::open(path).map_err(|source| RepositoryDiscoveryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = Vec::new();
    handle
        .take(MAX_GITFILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| RepositoryDiscoveryError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > MAX_GITFILE_BYTES {
        return Err(RepositoryDiscoveryError::GitFileTooLarge(
            path.to_path_buf(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| RepositoryDiscoveryError::MalformedGitFile(path.to_path_buf()))
}

fn nearest_workspace_root(start: &Path, repository: &Path) -> PathBuf {
    start
        .ancestors()
        .take_while(|candidate| candidate.starts_with(repository))
        .find(|candidate| {
            ["Cargo.toml", "package.json", "pyproject.toml", "go.work"]
                .iter()
                .any(|name| candidate.join(name).is_file())
        })
        .map_or_else(|| repository.to_path_buf(), Path::to_path_buf)
}

fn no_git(analysis_root: PathBuf, scope: AnalysisScope) -> DiscoveredRepository {
    build(
        analysis_root,
        None,
        None,
        None,
        RepositoryRootKind::NoGit,
        scope,
        None,
    )
}

fn build(
    analysis_root: PathBuf,
    worktree_root: Option<PathBuf>,
    git_dir: Option<PathBuf>,
    common_dir: Option<PathBuf>,
    kind: RepositoryRootKind,
    scope: AnalysisScope,
    diagnostic: Option<GitDiagnostic>,
) -> DiscoveredRepository {
    let root = RepositoryScopeRoot {
        analysis_root: normalize(&analysis_root),
        worktree_root: worktree_root.as_deref().map(normalize),
        git_dir: git_dir.as_deref().map(normalize),
        common_dir: common_dir.as_deref().map(normalize),
        kind,
        scope,
    };
    DiscoveredRepository {
        root,
        analysis_root,
        worktree_root,
        git_dir,
        common_dir,
        diagnostic,
    }
}

fn is_bare_repository(path: &Path) -> bool {
    path.join("HEAD").is_file() && path.join("objects").is_dir() && path.join("refs").is_dir()
}

fn discovery_diagnostic(error: &RepositoryDiscoveryError) -> GitDiagnostic {
    let (kind, path) = match error {
        RepositoryDiscoveryError::GitFileTooLarge(path)
        | RepositoryDiscoveryError::MalformedGitFile(path) => {
            (GitDiagnosticKind::MalformedGitFile, path)
        }
        RepositoryDiscoveryError::InvalidGitDirectory(path) => {
            (GitDiagnosticKind::EscapedMetadataBoundary, path)
        }
        RepositoryDiscoveryError::Io { path, source }
            if source.kind() == io::ErrorKind::PermissionDenied =>
        {
            (GitDiagnosticKind::PermissionDenied, path)
        }
        RepositoryDiscoveryError::Io { path, .. }
        | RepositoryDiscoveryError::NotDirectory(path) => (GitDiagnosticKind::Io, path),
    };
    GitDiagnostic {
        kind,
        path: normalize(path),
    }
}

fn normalize(path: &Path) -> String {
    normalize_windows_prefix(path.to_string_lossy().replace('\\', "/"))
}

fn normalize_windows_prefix(path: String) -> String {
    if let Some(rest) = path.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = path.strip_prefix("//?/") {
        rest.to_string()
    } else {
        path
    }
}
