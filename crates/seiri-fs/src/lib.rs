#![forbid(unsafe_code)]

use seiri_core::{FileRecord, IgnoredShallowRecord, ImportantFile};
use std::path::{Path, PathBuf};

mod classify;
mod containment;
mod walker;

pub use containment::{
    resolve_repository_path, RepositoryPathRejectReason, RepositoryPathResolution,
    RepositoryPathUnknownReason,
};

pub use walker::{
    resolve_repo_root, walk_repository, walk_repository_with_options, FsError, IgnorePolicy,
    RepositoryRoot, RepositoryWalk, RepositoryWalkSummary, ScanOptions, WalkCompletion,
    WalkLimitKind, WalkTruncation, DEFAULT_IGNORED_NAMES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoFsScan {
    pub repo_root: PathBuf,
    pub files: Vec<FileRecord>,
    pub important_files: Vec<ImportantFile>,
    pub walk_summary: RepositoryWalkSummary,
    pub ignored_shallow: Vec<IgnoredShallowRecord>,
}

pub fn scan_repository(path: impl AsRef<Path>) -> Result<RepoFsScan, FsError> {
    scan_repository_with_options(path, &ScanOptions::default())
}

pub fn scan_repository_with_options(
    path: impl AsRef<Path>,
    options: &ScanOptions,
) -> Result<RepoFsScan, FsError> {
    let walk = walk_repository_with_options(path, options)?;
    let important_files = classify::classify_important_files(walk.records());
    let (root, files, walk_summary, ignored_shallow) = walk.into_parts();
    Ok(RepoFsScan {
        repo_root: root.into_path_buf(),
        files,
        important_files,
        walk_summary,
        ignored_shallow,
    })
}
