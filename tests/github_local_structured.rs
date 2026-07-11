use seiri_core::{
    CoverageScope, CoverageStatus, DependencyBotProvider, GithubDiagnosticKind, GithubDocumentIr,
    GithubDocumentKind, GithubParseStatus, IssueFormFieldKind, StructuredBudgetKind,
};
use seiri_github_local::{parse_repository_github_documents_with_options, StructuredParseOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn parses_supported_local_github_documents_without_execution() {
    let repo = TempRepo::new("structured-success");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        ".github/issue_template/bug.yml",
        concat!(
            "name: Bug report\n",
            "description: Report a reproducible problem\n",
            "body:\n",
            "  - type: textarea\n",
            "    id: reproduction\n",
            "    validations:\n",
            "      required: true\n",
        ),
    );
    repo.write(
        ".github/workflows/ci.yml",
        concat!(
            "name: CI\n",
            "on:\n",
            "  push:\n",
            "jobs:\n",
            "  test:\n",
            "    runs-on: ubuntu-latest\n",
        ),
    );
    repo.write(
        ".github/dependabot.yml",
        concat!(
            "version: 2\n",
            "updates:\n",
            "  - package-ecosystem: cargo\n",
            "    directory: /\n",
            "    schedule:\n",
            "      interval: weekly\n",
        ),
    );
    repo.write(
        ".github/renovate.json",
        "{\"packageRules\":[{\"matchManagers\":[\"cargo\"],\"matchPaths\":[\"/\"],\"schedule\":[\"weekly\"]}]}",
    );
    repo.write("CODEOWNERS", "/src/ @maintainer @reviewer\n");

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit repository");
    assert_eq!(snapshot.github_local_documents.documents().len(), 5);
    assert!(snapshot
        .github_local_documents
        .documents()
        .iter()
        .all(|document| document.status == GithubParseStatus::Parsed));
    assert_eq!(
        snapshot
            .coverage
            .record(CoverageScope::DocumentRole(
                seiri_core::DocumentRole::GithubConfiguration
            ))
            .map(|record| record.status),
        Some(CoverageStatus::Complete)
    );

    let issue_form = document_ir(&snapshot, GithubDocumentKind::IssueForm);
    match issue_form {
        GithubDocumentIr::IssueForm(form) => {
            assert_eq!(form.name.as_deref(), Some("Bug report"));
            assert_eq!(form.fields.len(), 1);
            assert_eq!(form.fields[0].kind, IssueFormFieldKind::Textarea);
            assert_eq!(form.fields[0].id.as_deref(), Some("reproduction"));
            assert_eq!(form.fields[0].required, Some(true));
        }
        _ => panic!("expected issue form IR"),
    }

    let workflow = document_ir(&snapshot, GithubDocumentKind::Workflow);
    match workflow {
        GithubDocumentIr::Workflow(workflow) => {
            assert_eq!(workflow.name.as_deref(), Some("CI"));
            assert!(workflow
                .triggers
                .iter()
                .any(|trigger| trigger.name == "push"));
            assert!(workflow.jobs.iter().any(|job| job.id == "test"));
        }
        _ => panic!("expected workflow IR"),
    }

    let dependency_bots = snapshot
        .github_local_documents
        .documents()
        .iter()
        .filter_map(|document| document.ir.as_ref())
        .filter_map(|ir| match ir {
            GithubDocumentIr::DependencyBot(bot) => Some(bot),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(dependency_bots
        .iter()
        .any(|bot| bot.provider == DependencyBotProvider::Dependabot));
    assert!(dependency_bots
        .iter()
        .any(|bot| bot.provider == DependencyBotProvider::Renovate));
    assert!(dependency_bots.iter().all(|bot| !bot.updates.is_empty()));

    let codeowners = document_ir(&snapshot, GithubDocumentKind::Codeowners);
    match codeowners {
        GithubDocumentIr::Codeowners(codeowners) => {
            assert_eq!(codeowners.entries[0].pattern, "/src/");
            assert_eq!(codeowners.entries[0].owners, ["@maintainer", "@reviewer"]);
        }
        _ => panic!("expected CODEOWNERS IR"),
    }

    let json = seiri_report::to_json(&snapshot).expect("canonical JSON");
    let wire: serde_json::Value = serde_json::from_str(&json).expect("wire JSON");
    assert!(wire.get("github_local_documents").is_some());
}

#[test]
fn budgets_and_malformed_documents_remain_typed_and_span_aware() {
    let repo = TempRepo::new("structured-limits");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        ".github/workflows/ci.yml",
        concat!(
            "name: Continuous Integration\n",
            "on:\n",
            "  push:\n",
            "jobs:\n",
            "  test:\n",
        ),
    );
    repo.write("CODEOWNERS", "/src/\n");

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit repository");
    let source_limited = parse_repository_github_documents_with_options(
        repo.path(),
        &snapshot.document_index,
        &StructuredParseOptions {
            max_source_bytes: 4,
            ..StructuredParseOptions::default()
        },
    )
    .expect("source-limited parse");
    assert!(source_limited.documents().iter().all(|document| {
        document.status == GithubParseStatus::BudgetExceeded(StructuredBudgetKind::SourceBytes)
    }));

    let node_limited = parse_repository_github_documents_with_options(
        repo.path(),
        &snapshot.document_index,
        &StructuredParseOptions {
            max_nodes: 1,
            ..StructuredParseOptions::default()
        },
    )
    .expect("node-limited parse");
    assert!(node_limited.documents().iter().any(|document| {
        document.status == GithubParseStatus::BudgetExceeded(StructuredBudgetKind::Nodes)
    }));

    let depth_limited = parse_repository_github_documents_with_options(
        repo.path(),
        &snapshot.document_index,
        &StructuredParseOptions {
            max_depth: 0,
            ..StructuredParseOptions::default()
        },
    )
    .expect("depth-limited parse");
    assert!(depth_limited.documents().iter().any(|document| {
        document.status == GithubParseStatus::BudgetExceeded(StructuredBudgetKind::Depth)
    }));

    let scalar_limited = parse_repository_github_documents_with_options(
        repo.path(),
        &snapshot.document_index,
        &StructuredParseOptions {
            max_scalar_bytes: 2,
            ..StructuredParseOptions::default()
        },
    )
    .expect("scalar-limited parse");
    assert!(scalar_limited.documents().iter().any(|document| {
        document.status == GithubParseStatus::BudgetExceeded(StructuredBudgetKind::ScalarBytes)
    }));

    let diagnostic_limited = parse_repository_github_documents_with_options(
        repo.path(),
        &snapshot.document_index,
        &StructuredParseOptions {
            max_diagnostics: 0,
            ..StructuredParseOptions::default()
        },
    )
    .expect("diagnostic-limited parse");
    assert!(diagnostic_limited.documents().iter().any(|document| {
        document.status == GithubParseStatus::BudgetExceeded(StructuredBudgetKind::Diagnostics)
    }));

    let malformed = snapshot
        .github_local_documents
        .documents()
        .iter()
        .find(|document| document.kind == GithubDocumentKind::Codeowners)
        .expect("CODEOWNERS document");
    assert_eq!(malformed.status, GithubParseStatus::Malformed);
    assert_eq!(malformed.diagnostics.len(), 1);
    assert_eq!(
        malformed.diagnostics[0].kind,
        GithubDiagnosticKind::MissingCodeowner
    );
    assert_eq!(malformed.diagnostics[0].span.line, 1);
    assert_eq!(malformed.diagnostics[0].span.byte_start, 0);
    assert_eq!(malformed.diagnostics[0].span.byte_end, "/src/".len());
}

#[test]
fn unsupported_yaml_is_local_unknown_with_a_precise_span() {
    let repo = TempRepo::new("unsupported-yaml");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        ".github/workflows/unsupported.yml",
        concat!(
            "name: Unsupported\n",
            "jobs:\n",
            "  test:\n",
            "    runs-on: &runner ubuntu-latest\n",
        ),
    );

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit repository");
    let workflow = snapshot
        .github_local_documents
        .documents()
        .iter()
        .find(|document| document.kind == GithubDocumentKind::Workflow)
        .expect("workflow document");
    assert_eq!(workflow.status, GithubParseStatus::UnsupportedSyntax);
    assert_eq!(
        workflow.diagnostics[0].kind,
        GithubDiagnosticKind::UnsupportedSyntax
    );
    assert_eq!(workflow.diagnostics[0].span.line, 4);
    assert_eq!(
        snapshot
            .coverage
            .record(CoverageScope::DocumentRole(
                seiri_core::DocumentRole::GithubConfiguration
            ))
            .map(|record| record.status),
        Some(CoverageStatus::Partial(
            seiri_core::CoverageIncompleteReason::UnsupportedSyntax
        ))
    );
}

fn document_ir(
    snapshot: &seiri_core::RepositoryAnalysis,
    kind: GithubDocumentKind,
) -> &GithubDocumentIr {
    snapshot
        .github_local_documents
        .documents()
        .iter()
        .find(|document| document.kind == kind)
        .and_then(|document| document.ir.as_ref())
        .expect("parsed GitHub document IR")
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "reposeiri-github-local-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp repo");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, content).expect("write fixture");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
