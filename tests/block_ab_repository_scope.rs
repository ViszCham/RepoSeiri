use seiri_core::{
    AnalysisScope, CoverageIncompleteReason, CoverageStatus, GitObservationState, GitReadBudget,
    GitTemporalObservation, ManifestObservationStatus, RepositoryRootKind, ScopeNodeKind,
};
use seiri_git_local::{
    analyze_discovered_repository, discover_repository, GitReadBackend, GixReadBackend,
    RepositoryAnalysisOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn repository_discovery_prefers_git_root_and_requires_explicit_subtree() {
    let root = temp_root("discovery");
    create_minimal_git_dir(&root.join(".git"));
    fs::create_dir_all(root.join("packages/demo")).expect("package");
    fs::write(root.join("packages/demo/README.md"), "# Demo\n").expect("README");

    let repository = discover_repository(&root.join("packages/demo"), AnalysisScope::Repository)
        .expect("repository discovery");
    assert_eq!(repository.analysis_root(), fs::canonicalize(&root).unwrap());
    assert_eq!(repository.root().kind, RepositoryRootKind::Worktree);

    let subtree = discover_repository(&root.join("packages/demo"), AnalysisScope::Subtree)
        .expect("subtree discovery");
    assert_eq!(
        subtree.analysis_root(),
        fs::canonicalize(root.join("packages/demo")).unwrap()
    );
    assert_eq!(subtree.root().kind, RepositoryRootKind::Subtree);
    cleanup(root);
}

#[test]
fn linked_worktree_and_malformed_gitfile_are_typed_without_shelling_out() {
    let root = temp_root("gitfile");
    let metadata = root.join("metadata");
    create_minimal_git_dir(&metadata);
    let worktree = root.join("worktree");
    fs::create_dir_all(&worktree).expect("worktree");
    fs::write(worktree.join(".git"), "gitdir: ../metadata\n").expect("gitfile");
    let linked = discover_repository(&worktree, AnalysisScope::Repository).expect("linked");
    assert_eq!(linked.root().kind, RepositoryRootKind::LinkedWorktree);
    assert_eq!(
        linked.git_dir(),
        Some(fs::canonicalize(metadata).unwrap().as_path())
    );

    let malformed = root.join("malformed");
    fs::create_dir_all(&malformed).expect("malformed root");
    fs::write(malformed.join(".git"), "not-a-gitfile\n").expect("bad gitfile");
    let malformed =
        discover_repository(&malformed, AnalysisScope::Repository).expect("typed malformed");
    assert_eq!(malformed.root().kind, RepositoryRootKind::MalformedGit);
    let observed = GixReadBackend.observe(&malformed, GitReadBudget::default());
    assert_eq!(observed.state, GitObservationState::Unknown);
    assert_eq!(
        observed.refs_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::ParseFailed)
    );
    cleanup(root);
}

#[test]
fn shallow_refs_and_alternates_keep_bounded_coverage_visible() {
    let root = temp_root("git-coverage");
    let git = root.join(".git");
    create_minimal_git_dir(&git);
    fs::write(
        git.join("shallow"),
        "0000000000000000000000000000000000000000\n",
    )
    .expect("shallow");
    fs::write(
        git.join("refs/heads/main"),
        "1111111111111111111111111111111111111111\n",
    )
    .expect("main ref");
    fs::write(
        git.join("refs/heads/other"),
        "2222222222222222222222222222222222222222\n",
    )
    .expect("other ref");
    let discovered = discover_repository(&root, AnalysisScope::Repository).expect("discover");
    let observed = GixReadBackend.observe(
        &discovered,
        GitReadBudget {
            max_refs: 1,
            max_tags: 1,
            max_commit_headers: 1,
        },
    );
    assert!(observed.shallow);
    assert_eq!(
        observed.refs_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    );
    assert_eq!(
        observed.commits_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::Unavailable)
    );

    fs::create_dir_all(git.join("objects/info")).expect("object info");
    fs::write(git.join("objects/info/alternates"), "../../outside\n").expect("alternates");
    let blocked = GixReadBackend.observe(&discovered, GitReadBudget::default());
    assert_eq!(blocked.state, GitObservationState::Unknown);
    assert!(blocked.references.is_empty());

    fs::remove_file(git.join("objects/info/alternates")).expect("remove alternates");
    fs::File::create(git.join("packed-refs"))
        .expect("packed refs")
        .set_len(16 * 1024 * 1024 + 1)
        .expect("oversized packed refs");
    let oversized = GixReadBackend.observe(&discovered, GitReadBudget::default());
    assert_eq!(oversized.state, GitObservationState::Unknown);
    assert_eq!(
        oversized.refs_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    );
    assert!(oversized.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == seiri_core::GitDiagnosticKind::PackedReferencesTooLarge
    }));
    cleanup(root);
}

#[test]
fn zero_budgets_fail_closed_without_panicking() {
    let root = temp_root("zero-budgets");
    let git = root.join(".git");
    create_minimal_git_dir(&git);
    fs::write(
        git.join("refs/heads/main"),
        "1111111111111111111111111111111111111111\n",
    )
    .expect("main ref");
    write(&root, "Cargo.toml", "[package]\nname = \"zero\"\n");

    let discovered = discover_repository(&root, AnalysisScope::Repository).expect("discover");
    let observed = GixReadBackend.observe(
        &discovered,
        GitReadBudget {
            max_refs: 0,
            max_tags: 0,
            max_commit_headers: 0,
        },
    );
    assert_eq!(
        observed.refs_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    );

    let scan = seiri_fs::scan_repository(&root).expect("filesystem scan");
    let report = analyze_discovered_repository(
        &discovered,
        &scan.files,
        &scan.ignored_shallow,
        scan.walk_summary.ignored_records_truncated,
        RepositoryAnalysisOptions {
            graph: seiri_core::ScopeReadBudget {
                max_nodes: 0,
                ..seiri_core::ScopeReadBudget::default()
            },
            ..RepositoryAnalysisOptions::default()
        },
        &NoGitBackend,
    );
    assert!(report.graph.nodes.is_empty());
    assert_eq!(
        report.graph.node_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    );
    cleanup(root);
}

#[test]
fn workspace_scope_graph_preserves_packages_docs_fixtures_submodules_and_ignored_shallow() {
    let root = temp_root("scope-graph");
    write(
        &root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n",
    );
    write(
        &root,
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
    );
    write(
        &root,
        "web/package.json",
        "{\"name\":\"web\",\"workspaces\":[\"plugins/*\"]}\n",
    );
    write(&root, "plugins/p/package.json", "{\"name\":\"p\"}\n");
    write(&root, "docs/guide.md", "# Guide\n");
    write(&root, "examples/demo.txt", "example\n");
    write(&root, "fixtures/case/input.txt", "fixture\n");
    write(
        &root,
        ".gitmodules",
        "[submodule \"vendor/lib\"]\n\tpath = vendor/lib\n\turl = local-only\n",
    );
    write(&root, "target/private/secret.txt", "not traversed\n");

    let scan = seiri_fs::scan_repository(&root).expect("filesystem scan");
    let discovered = discover_repository(&root, AnalysisScope::Repository).expect("discover");
    let report = analyze_discovered_repository(
        &discovered,
        &scan.files,
        &scan.ignored_shallow,
        scan.walk_summary.ignored_records_truncated,
        RepositoryAnalysisOptions::default(),
        &NoGitBackend,
    );
    for kind in [
        ScopeNodeKind::Repository,
        ScopeNodeKind::Workspace,
        ScopeNodeKind::Package,
        ScopeNodeKind::Documentation,
        ScopeNodeKind::Example,
        ScopeNodeKind::Fixture,
        ScopeNodeKind::Submodule,
    ] {
        assert!(
            report.graph.nodes.iter().any(|node| node.kind == kind),
            "{kind:?}"
        );
    }
    assert!(report
        .graph
        .ignored
        .iter()
        .any(|record| record.path == "target"));
    assert!(!report
        .graph
        .ignored
        .iter()
        .any(|record| record.path.contains("secret.txt")));
    assert!(report
        .graph
        .edges
        .iter()
        .any(|edge| { edge.kind == seiri_core::ScopeEdgeKind::DeclaresMember }));

    let limited = analyze_discovered_repository(
        &discovered,
        &scan.files,
        &scan.ignored_shallow,
        false,
        RepositoryAnalysisOptions {
            graph: seiri_core::ScopeReadBudget {
                max_manifest_bytes: 4,
                ..seiri_core::ScopeReadBudget::default()
            },
            ..RepositoryAnalysisOptions::default()
        },
        &NoGitBackend,
    );
    assert!(limited
        .graph
        .manifests
        .iter()
        .any(|manifest| { manifest.status == ManifestObservationStatus::SourceTooLarge }));
    assert_eq!(
        limited.graph.manifest_coverage,
        CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded)
    );
    cleanup(root);
}

#[test]
fn freshness_dimensions_do_not_promote_timestamps_to_lifecycle_state() {
    let root = temp_root("freshness");
    write(
        &root,
        "README.md",
        "# Demo\n\n[Docs](docs/guide.md)\n[Missing](docs/missing.md)\n\n## Maintenance status\nExperimental.\n",
    );
    write(&root, "docs/guide.md", "# Guide\n");
    let snapshot = seiri_report::audit_repository(&root).expect("audit");
    assert_eq!(
        snapshot.freshness.temporal_activity.coverage,
        CoverageStatus::NotRequested
    );
    assert!(snapshot.freshness.lifecycle_signal.route_state.is_some());
    assert_eq!(
        snapshot.freshness.temporal_activity.observed_commit_headers,
        0
    );
    assert!(
        snapshot
            .freshness
            .target_reachability
            .repository_local_present
            > 0
    );
    assert!(seiri_report::to_markdown(&snapshot).contains("Freshness Dimensions"));
    cleanup(root);
}

struct NoGitBackend;

impl GitReadBackend for NoGitBackend {
    fn observe(
        &self,
        _root: &seiri_git_local::DiscoveredRepository,
        _budget: GitReadBudget,
    ) -> GitTemporalObservation {
        GitTemporalObservation::default()
    }
}

fn create_minimal_git_dir(path: &Path) {
    fs::create_dir_all(path.join("objects")).expect("objects");
    fs::create_dir_all(path.join("refs/heads")).expect("refs");
    fs::write(path.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
}

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent");
    }
    fs::write(path, body).expect("fixture write");
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("reposeiri-ab-{label}-{nonce}"));
    fs::create_dir_all(&root).expect("temp root");
    root
}

fn cleanup(path: PathBuf) {
    fs::remove_dir_all(path).expect("cleanup");
}
