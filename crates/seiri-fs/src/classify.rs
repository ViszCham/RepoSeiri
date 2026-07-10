use seiri_core::{FileKind, FileRecord, ImportantFile, ImportantFileKind};

pub(crate) fn classify_important_files(records: &[FileRecord]) -> Vec<ImportantFile> {
    let mut important_files = records
        .iter()
        .filter_map(|record| important_file(record).map(|kind| (record.path.clone(), kind)))
        .map(|(path, kind)| ImportantFile { path, kind })
        .collect::<Vec<_>>();
    important_files.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.path.cmp(&right.path))
    });
    important_files
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
        (FileKind::File, ".gitignore", _) => Some(ImportantFileKind::Gitignore),
        (FileKind::File, ".gitattributes", _) => Some(ImportantFileKind::Gitattributes),
        (FileKind::File, ".editorconfig", _) => Some(ImportantFileKind::EditorConfig),
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
