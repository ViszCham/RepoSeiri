use seiri_core::{FileKind, FileRecord, ImportantFile, ImportantFileKind};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    pub max_depth: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self { max_depth: 32 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoFsScan {
    pub repo_root: PathBuf,
    pub files: Vec<FileRecord>,
    pub important_files: Vec<ImportantFile>,
}

#[derive(Debug)]
pub enum FsError {
    Io { path: PathBuf, source: io::Error },
    NotDirectory(PathBuf),
}

impl Display for FsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            Self::NotDirectory(path) => write!(f, "{} is not a directory", path.display()),
        }
    }
}

impl std::error::Error for FsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::NotDirectory(_) => None,
        }
    }
}

pub fn scan_repository(path: impl AsRef<Path>) -> Result<RepoFsScan, FsError> {
    scan_repository_with_options(path, &ScanOptions::default())
}

pub fn scan_repository_with_options(
    path: impl AsRef<Path>,
    options: &ScanOptions,
) -> Result<RepoFsScan, FsError> {
    let repo_root = resolve_repo_root(path.as_ref())?;
    let mut files = Vec::new();
    walk_dir(&repo_root, &repo_root, 0, options.max_depth, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let mut important_files = files
        .iter()
        .filter_map(|record| important_file(record).map(|kind| (record.path.clone(), kind)))
        .map(|(path, kind)| ImportantFile { path, kind })
        .collect::<Vec<_>>();
    important_files.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(RepoFsScan {
        repo_root,
        files,
        important_files,
    })
}

pub fn resolve_repo_root(path: &Path) -> Result<PathBuf, FsError> {
    let canonical = fs::canonicalize(path).map_err(|source| FsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !canonical.is_dir() {
        return Err(FsError::NotDirectory(canonical));
    }
    if has_repo_boundary_marker(&canonical) {
        return Ok(canonical);
    }

    let mut cursor = Some(canonical.as_path());
    while let Some(candidate) = cursor {
        if candidate.join(".git").exists() {
            return Ok(candidate.to_path_buf());
        }
        cursor = candidate.parent();
    }

    Ok(canonical)
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

fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    records: &mut Vec<FileRecord>,
) -> Result<(), FsError> {
    if depth > max_depth {
        return Ok(());
    }

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
        if should_ignore_name(&name.to_string_lossy()) {
            continue;
        }

        let file_type = entry.file_type().map_err(|source| FsError::Io {
            path: path.clone(),
            source,
        })?;
        let metadata = entry.metadata().map_err(|source| FsError::Io {
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

        records.push(FileRecord {
            path: normalize_relative_path(root, &path),
            kind,
            size_bytes: if matches!(kind, FileKind::File) {
                metadata.len()
            } else {
                0
            },
        });

        if file_type.is_dir() {
            walk_dir(root, &path, depth + 1, max_depth, records)?;
        }
    }

    Ok(())
}

fn should_ignore_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | "target"
            | "node_modules"
            | ".venv"
            | "dist"
            | "build"
            | ".idea"
            | ".vscode"
    )
}

fn important_file(record: &FileRecord) -> Option<ImportantFileKind> {
    let path = record.path.replace('\\', "/");
    let lower = path.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or(&lower);

    if record.kind == FileKind::File {
        if is_issue_form_path(&lower) {
            return Some(ImportantFileKind::IssueForm);
        }
        if is_issue_template_path(&lower) {
            return Some(ImportantFileKind::IssueTemplate);
        }
        if is_pull_request_template_path(&lower) {
            return Some(ImportantFileKind::PullRequestTemplate);
        }
        if is_dependency_bot_path(&lower) {
            return Some(ImportantFileKind::DependencyBot);
        }
        if is_security_automation_path(&lower) {
            return Some(ImportantFileKind::SecurityAutomation);
        }
    }

    match (record.kind, basename, lower.as_str()) {
        (FileKind::File, name, _) if name.starts_with("readme") => Some(ImportantFileKind::Readme),
        (FileKind::File, "license" | "license.md" | "copying", _) => {
            Some(ImportantFileKind::License)
        }
        (FileKind::File, "contributing.md" | "contributing", _) => {
            Some(ImportantFileKind::Contributing)
        }
        (FileKind::File, "security.md" | "security", _) => Some(ImportantFileKind::Security),
        (FileKind::File, "support.md" | "support", _) => Some(ImportantFileKind::Support),
        (FileKind::File, "changelog.md" | "changelog" | "changes.md", _) => {
            Some(ImportantFileKind::Changelog)
        }
        (FileKind::File, "codeowners", _) => Some(ImportantFileKind::Codeowners),
        (FileKind::File, "cargo.toml", _) => Some(ImportantFileKind::CargoToml),
        (FileKind::Directory, "docs", _) => Some(ImportantFileKind::DocsDirectory),
        (FileKind::File, _, value)
            if value.starts_with(".github/workflows/")
                && (value.ends_with(".yml") || value.ends_with(".yaml")) =>
        {
            Some(ImportantFileKind::Workflow)
        }
        _ => None,
    }
}

fn is_issue_form_path(path: &str) -> bool {
    path.starts_with(".github/issue_template/")
        && !is_issue_template_config(path)
        && (path.ends_with(".yml") || path.ends_with(".yaml"))
}

fn is_issue_template_path(path: &str) -> bool {
    matches!(path, "issue_template.md" | ".github/issue_template.md")
        || (path.starts_with(".github/issue_template/")
            && !is_issue_template_config(path)
            && path.ends_with(".md"))
}

fn is_issue_template_config(path: &str) -> bool {
    path.ends_with("/config.yml") || path.ends_with("/config.yaml")
}

fn is_pull_request_template_path(path: &str) -> bool {
    matches!(
        path,
        "pull_request_template.md" | ".github/pull_request_template.md"
    ) || path.starts_with(".github/pull_request_template/")
}

fn is_dependency_bot_path(path: &str) -> bool {
    matches!(
        path,
        ".github/dependabot.yml"
            | ".github/dependabot.yaml"
            | "renovate.json"
            | ".github/renovate.json"
            | ".renovaterc"
            | ".renovaterc.json"
    )
}

fn is_security_automation_path(path: &str) -> bool {
    path.starts_with(".github/workflows/")
        && (path.ends_with(".yml") || path.ends_with(".yaml"))
        && path.rsplit('/').next().is_some_and(|name| {
            name.contains("codeql")
                || name.contains("security")
                || name.contains("scorecard")
                || name.contains("sast")
                || name.contains("govulncheck")
                || name.contains("vuln")
                || name.contains("fuzz")
        })
}

fn normalize_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
