#![forbid(unsafe_code)]

use seiri_core::{
    AnalysisScope, FileRecord, GitReadBudget, IgnoredShallowRecord, RepositoryScopeReport,
    ScopeReadBudget,
};
use std::fmt::{Display, Formatter};
use std::path::Path;

mod discovery;
mod git;
mod scope;

pub use discovery::{discover_repository, DiscoveredRepository, RepositoryDiscoveryError};
pub use git::{GitReadBackend, GixReadBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepositoryAnalysisOptions {
    pub scope: AnalysisScope,
    pub git: GitReadBudget,
    pub graph: ScopeReadBudget,
}

impl Default for RepositoryAnalysisOptions {
    fn default() -> Self {
        Self {
            scope: AnalysisScope::Repository,
            git: GitReadBudget::default(),
            graph: ScopeReadBudget::default(),
        }
    }
}

#[derive(Debug)]
pub enum GitLocalError {
    Discovery(RepositoryDiscoveryError),
}

impl Display for GitLocalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovery(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for GitLocalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Discovery(error) => Some(error),
        }
    }
}

impl From<RepositoryDiscoveryError> for GitLocalError {
    fn from(value: RepositoryDiscoveryError) -> Self {
        Self::Discovery(value)
    }
}

pub fn analyze_repository_scope(
    input: &Path,
    files: &[FileRecord],
    ignored: &[IgnoredShallowRecord],
    ignored_truncated: bool,
    options: RepositoryAnalysisOptions,
) -> Result<RepositoryScopeReport, GitLocalError> {
    let discovered = discover_repository(input, options.scope)?;
    Ok(analyze_discovered_repository(
        &discovered,
        files,
        ignored,
        ignored_truncated,
        options,
        &GixReadBackend,
    ))
}

pub fn analyze_discovered_repository<B: GitReadBackend>(
    discovered: &DiscoveredRepository,
    files: &[FileRecord],
    ignored: &[IgnoredShallowRecord],
    ignored_truncated: bool,
    options: RepositoryAnalysisOptions,
    backend: &B,
) -> RepositoryScopeReport {
    RepositoryScopeReport {
        root: discovered.root().clone(),
        git: backend.observe(discovered, options.git),
        graph: scope::build_scope_graph(
            discovered.analysis_root(),
            files,
            ignored,
            ignored_truncated,
            options.graph,
        ),
    }
}
