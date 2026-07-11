use seiri_core::{
    ActionReferenceKind, CoverageStatus, CriticalPathKind, GithubDiagnosticKind, GithubDocumentIr,
    GithubDocumentKind, GithubParseStatus, IssueFormFieldKind, IssueRouteCandidateKind,
    ProfileKind, StaticUnknownReason, StaticValue, TokenPermission, WorkflowJobCandidateKind,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn ac0_ac4_workflow_keeps_dynamic_values_permissions_and_action_refs_typed() {
    let repo = TempRepo::new("workflow");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        ".github/workflows/ci.yml",
        concat!(
            "name: CI\n",
            "on: [push, pull_request]\n",
            "jobs:\n",
            "  test:\n",
            "    name: Test and lint\n",
            "    permissions:\n",
            "      contents: read\n",
            "      checks: ${{ matrix.permission }}\n",
            "    steps:\n",
            "      - uses: actions/checkout@0123456789abcdef0123456789abcdef01234567\n",
            "      - name: Dynamic action\n",
            "        uses: ${{ matrix.action }}\n",
            "      - uses: docker://alpine:3.20\n",
            "      - uses: actions/setup-node@v4\n",
            "      - run: |\n",
            "          cargo test\n",
            "  deploy:\n",
            "    uses: ./.github/workflows/deploy.yml\n",
        ),
    );

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit");
    let workflow = workflow(&snapshot);
    assert_eq!(
        workflow.permissions.default,
        TokenPermission::DefaultOrInheritedUnknown
    );
    assert!(workflow
        .triggers
        .iter()
        .all(|trigger| trigger.value.is_literal()));

    let test = workflow.jobs.iter().find(|job| job.id == "test").unwrap();
    assert_eq!(test.permissions.default, TokenPermission::None);
    assert!(matches!(
        test.permissions.entries[0].permission,
        StaticValue::Literal(TokenPermission::Read)
    ));
    assert!(matches!(
        test.permissions.entries[1].permission,
        StaticValue::Expression { .. }
    ));
    assert!(matches!(
        test.steps[0].uses.as_ref().unwrap().kind,
        ActionReferenceKind::FullObjectId(_)
    ));
    assert!(matches!(
        test.steps[1].uses.as_ref().unwrap().kind,
        ActionReferenceKind::Dynamic
    ));
    assert!(matches!(
        test.steps[2].uses.as_ref().unwrap().kind,
        ActionReferenceKind::Docker(_)
    ));
    assert!(matches!(
        test.steps[3].uses.as_ref().unwrap().kind,
        ActionReferenceKind::TagOrBranch(_)
    ));
    assert!(test.steps[4].has_run_script);
    assert!(test
        .candidates
        .iter()
        .any(|candidate| candidate.kind == WorkflowJobCandidateKind::Test));
    assert!(test
        .candidates
        .iter()
        .any(|candidate| candidate.kind == WorkflowJobCandidateKind::Lint));

    let deploy = workflow.jobs.iter().find(|job| job.id == "deploy").unwrap();
    assert!(matches!(
        deploy.reusable_workflow.as_ref().unwrap().kind,
        ActionReferenceKind::LocalPath(_)
    ));
    assert_eq!(
        deploy.permissions.default,
        TokenPermission::DefaultOrInheritedUnknown
    );
    assert!(deploy
        .candidates
        .iter()
        .any(|candidate| candidate.kind == WorkflowJobCandidateKind::Deploy));
}

#[test]
fn ac5_issue_forms_preserve_unknown_fields_and_static_route_candidates() {
    let repo = TempRepo::new("issue-form");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        ".github/issue_template/security.yml",
        concat!(
            "name: Security question and vulnerability support\n",
            "future-key: retained\n",
            "body:\n",
            "  - type: future-widget\n",
            "    id: evidence\n",
            "    future-input-key: retained\n",
            "    validations:\n",
            "      required: true\n",
            "  - type: upload\n",
            "    id: screenshot\n",
            "    validations:\n",
            "      required: false\n",
        ),
    );

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit");
    let form = issue_form(&snapshot);
    assert!(form.required_fields.name);
    assert!(!form.required_fields.description);
    assert!(form.required_fields.body);
    assert_eq!(form.unknown_top_level_keys, ["future-key"]);
    assert_eq!(form.fields[0].kind, IssueFormFieldKind::Unknown);
    assert_eq!(form.fields[0].unknown_keys, ["future-input-key"]);
    assert_eq!(form.fields[1].kind, IssueFormFieldKind::Upload);
    assert!(form
        .route_candidates
        .iter()
        .any(|candidate| candidate.kind == IssueRouteCandidateKind::Security));
    assert!(form
        .route_candidates
        .iter()
        .any(|candidate| candidate.kind == IssueRouteCandidateKind::Question));
    let document = document(&snapshot, GithubDocumentKind::IssueForm);
    assert_eq!(document.status, GithubParseStatus::ParsedPartial);
    assert!(document
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.kind == GithubDiagnosticKind::UnknownField));
    assert!(document
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.kind == GithubDiagnosticKind::MissingRequiredField));
}

#[test]
fn ac6_codeowners_compiles_supported_patterns_and_retains_skipped_lines() {
    let repo = TempRepo::new("codeowners");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        "docs/CODEOWNERS",
        "/src/**/tests/*.rs @owner\n!/secret @owner\n[ab].rs @owner\n",
    );

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit");
    let codeowners_document = document(&snapshot, GithubDocumentKind::Codeowners);
    assert_eq!(codeowners_document.status, GithubParseStatus::ParsedPartial);
    let owners = match codeowners_document.ir.as_ref().unwrap() {
        GithubDocumentIr::Codeowners(owners) => owners,
        _ => panic!("CODEOWNERS IR"),
    };
    assert_eq!(owners.entries.len(), 1);
    assert!(!owners.entries[0].program.ops.is_empty());
    assert_eq!(owners.entries[0].program.owners, ["@owner"]);
    assert_eq!(owners.skipped.len(), 2);
    assert!(owners.skipped.iter().all(|line| {
        line.diagnostic == GithubDiagnosticKind::UnsupportedCodeownersPattern && line.span.line > 1
    }));
}

#[test]
fn ac7_ac8_dependency_and_scope_coverage_remain_static_and_bounded() {
    let repo = TempRepo::new("dependency");
    repo.write("README.md", "# Fixture\n");
    repo.write("docs/guide.md", "# Guide\n");
    repo.write("SECURITY.md", "# Security\n");
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    );
    repo.write(
        ".github/dependabot.yml",
        concat!(
            "version: 2\n",
            "updates:\n",
            "  - package-ecosystem: cargo\n",
            "    directories:\n",
            "      - /\n",
            "      - /tools\n",
            "    schedule:\n",
            "      interval: weekly\n",
            "    open-pull-requests-limit: 7\n",
        ),
    );

    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit");
    let update = match document(&snapshot, GithubDocumentKind::DependencyBot)
        .ir
        .as_ref()
        .unwrap()
    {
        GithubDocumentIr::DependencyBot(bot) => &bot.updates[0],
        _ => panic!("dependency bot IR"),
    };
    assert!(matches!(
        update.ecosystem_value,
        StaticValue::Literal(ref value) if value == "cargo"
    ));
    assert_eq!(update.directory_values.len(), 2);
    assert!(matches!(
        update.schedule_value,
        StaticValue::Literal(ref value) if value == "weekly"
    ));
    assert_eq!(update.open_pull_requests_limit, StaticValue::Literal(7));

    for kind in [
        CriticalPathKind::DependencyAutomation,
        CriticalPathKind::Security,
        CriticalPathKind::Documentation,
        CriticalPathKind::Manifest,
    ] {
        let coverage = snapshot
            .github_semantics
            .critical_paths
            .iter()
            .find(|coverage| coverage.kind == kind)
            .expect("critical path");
        assert_eq!(coverage.coverage, CoverageStatus::Complete);
        assert!(coverage.observed > 0, "{kind:?}");
    }
    let markdown = seiri_report::to_markdown(&snapshot);
    assert!(markdown.contains("Structured GitHub Semantics v2"));
    assert!(markdown.contains("do not establish workflow success"));
    let native = seiri_report::codex_native_v3_query_repository_to_json(
        repo.path(),
        ProfileKind::Common,
        seiri_codex::CodexNativeV3QueryKind::Documents,
    )
    .expect("native documents query");
    assert!(native.contains("github_semantics"));
}

#[test]
fn ac0_dynamic_dependency_values_never_become_literals() {
    let repo = TempRepo::new("dynamic-dependency");
    repo.write("README.md", "# Fixture\n");
    repo.write(
        ".github/dependabot.yml",
        concat!(
            "version: 2\n",
            "updates:\n",
            "  - package-ecosystem: ${{ matrix.ecosystem }}\n",
            "    directory: /\n",
            "    schedule:\n",
            "      interval: weekly\n",
        ),
    );
    let snapshot = seiri_report::audit_repository(repo.path()).expect("audit");
    let update = match document(&snapshot, GithubDocumentKind::DependencyBot)
        .ir
        .as_ref()
        .unwrap()
    {
        GithubDocumentIr::DependencyBot(bot) => &bot.updates[0],
        _ => panic!("dependency bot IR"),
    };
    assert!(matches!(
        update.ecosystem_value,
        StaticValue::Expression { .. }
    ));
    assert_eq!(
        update.open_pull_requests_limit,
        StaticValue::Unknown(StaticUnknownReason::Omitted)
    );
}

fn document(
    snapshot: &seiri_core::RepoSnapshot,
    kind: GithubDocumentKind,
) -> &seiri_core::GithubLocalDocument {
    snapshot
        .github_local_documents
        .documents()
        .iter()
        .find(|document| document.kind == kind)
        .expect("GitHub document")
}

fn workflow(snapshot: &seiri_core::RepoSnapshot) -> &seiri_core::Workflow {
    match document(snapshot, GithubDocumentKind::Workflow)
        .ir
        .as_ref()
        .unwrap()
    {
        GithubDocumentIr::Workflow(workflow) => workflow,
        _ => panic!("workflow IR"),
    }
}

fn issue_form(snapshot: &seiri_core::RepoSnapshot) -> &seiri_core::IssueForm {
    match document(snapshot, GithubDocumentKind::IssueForm)
        .ir
        .as_ref()
        .unwrap()
    {
        GithubDocumentIr::IssueForm(form) => form,
        _ => panic!("issue form IR"),
    }
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "reposeiri-ac-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp repo");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, content).expect("write fixture");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
