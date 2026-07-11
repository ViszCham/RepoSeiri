use seiri_core::{
    CodexNativeReviewContext, PatchPreflightCheckKind, PatchPreflightStatus, PatchProposalDecision,
    PatchProposalIssueKind, ProfileKind, RemoteEvidenceStatus, CODEX_NATIVE_V3_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

struct TempFixture {
    root: PathBuf,
}

impl TempFixture {
    fn copy_safe_plan_repo() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "reposeiri-q34-{}-{}-{}",
            std::process::id(),
            epoch,
            sequence,
        ));
        fs::create_dir_all(root.join("docs")).expect("create fixture docs");
        let source = fixture("safe-plan-repo");
        fs::copy(source.join("README.md"), root.join("README.md")).expect("copy README");
        fs::copy(source.join("LICENSE"), root.join("LICENSE")).expect("copy LICENSE");
        fs::copy(
            source.join("docs").join("index.md"),
            root.join("docs").join("index.md"),
        )
        .expect("copy docs index");
        Self { root }
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn q33_native_v3_queries_borrow_canonical_collections_and_keep_v1_v2_wire_clean() {
    let root = fixture("safe-plan-repo");
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common)
        .expect("audit fixture");
    let bound_plan = seiri_planner::plan_safe_patches(&snapshot);
    let view = seiri_codex::CodexNativeV3View::new(&snapshot, &bound_plan, None);

    let routes = view.query(seiri_codex::CodexNativeV3QueryKind::Routes);
    assert_eq!(routes.schema_version, CODEX_NATIVE_V3_SCHEMA_VERSION);
    let seiri_codex::CodexNativeV3Query::Routes(route_data) = &routes.query else {
        panic!("native v3 route query");
    };
    assert_eq!(
        route_data.assessments.as_ptr(),
        snapshot.route_assessments.as_ptr(),
        "borrowed v3 routes must not clone the canonical collection"
    );

    let patches = view.query(seiri_codex::CodexNativeV3QueryKind::Patches);
    let patch_value = serde_json::to_value(&patches).expect("native v3 patch JSON");
    assert!(patch_value["query"]["data"].get("analysis_run").is_some());
    assert!(patch_value["query"]["data"]
        .get("operation_bindings")
        .is_some());

    let legacy_plan = seiri_planner::plan_compatibility_safe_patches(&snapshot);
    let legacy_kernel = seiri_codex::build_review_kernel(&snapshot, &legacy_plan, None);
    let compatibility_json =
        seiri_report::codex_to_json(&legacy_kernel.compatibility_v1()).expect("compatibility JSON");
    let native_v2: CodexNativeReviewContext = legacy_kernel.native_v2();
    let native_v2_json = seiri_report::codex_native_to_json(&native_v2).expect("native v2 JSON");
    assert!(!compatibility_json.contains("analysis_run"));
    assert!(!native_v2_json.contains("analysis_run"));
    assert!(!native_v2_json.contains("operation_bindings"));
}

#[test]
fn block_y_query_registry_and_borrowed_variants_cover_all_nine() {
    let root = fixture("safe-plan-repo");
    let snapshot = seiri_report::audit_repository_with_profile(&root, ProfileKind::Common)
        .expect("audit fixture");
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    let lint = seiri_report::lint_wording_repository_with_profile(&root, ProfileKind::Common)
        .expect("lint fixture");
    let view = seiri_codex::CodexNativeV3View::new(&snapshot, &plan, Some(&lint));

    let expected = [
        "summary",
        "routes",
        "evidence",
        "documents",
        "governance",
        "patches",
        "linter",
        "actions",
        "remote",
    ];
    for (kind, slug) in seiri_codex::CodexNativeV3QueryKind::ALL
        .into_iter()
        .zip(expected)
    {
        assert_eq!(kind.slug(), slug);
        assert_eq!(
            slug.parse::<seiri_codex::CodexNativeV3QueryKind>(),
            Ok(kind)
        );
    }
    assert!("unknown"
        .parse::<seiri_codex::CodexNativeV3QueryKind>()
        .is_err());

    let routes = view.query(seiri_codex::CodexNativeV3QueryKind::Routes);
    let seiri_codex::CodexNativeV3Query::Routes(data) = routes.query else {
        panic!("routes query");
    };
    assert!(std::ptr::eq(
        data.assessments,
        snapshot.route_assessments.as_slice()
    ));
    assert!(std::ptr::eq(
        data.missing_route_priority,
        &snapshot.missing_route_priority
    ));

    let evidence = view.query(seiri_codex::CodexNativeV3QueryKind::Evidence);
    let seiri_codex::CodexNativeV3Query::Evidence(data) = evidence.query else {
        panic!("evidence query");
    };
    assert!(std::ptr::eq(data.kernel, &snapshot.evidence_kernel_v2));
    assert!(std::ptr::eq(data.coverage, &snapshot.coverage));

    let documents = view.query(seiri_codex::CodexNativeV3QueryKind::Documents);
    let seiri_codex::CodexNativeV3Query::Documents(data) = documents.query else {
        panic!("documents query");
    };
    assert!(std::ptr::eq(data.index, &snapshot.document_index));
    assert!(std::ptr::eq(
        data.github_local,
        &snapshot.github_local_documents
    ));

    let governance = view.query(seiri_codex::CodexNativeV3QueryKind::Governance);
    let seiri_codex::CodexNativeV3Query::Governance(data) = governance.query else {
        panic!("governance query");
    };
    assert!(std::ptr::eq(data.facets, &snapshot.facets));
    assert!(std::ptr::eq(
        data.consistency,
        &snapshot.document_consistency
    ));
    assert!(std::ptr::eq(
        data.route_content,
        snapshot.route_content.as_slice()
    ));

    let patches = view.query(seiri_codex::CodexNativeV3QueryKind::Patches);
    let seiri_codex::CodexNativeV3Query::Patches(data) = patches.query else {
        panic!("patches query");
    };
    assert!(std::ptr::eq(data.plan, &plan));

    let linter = view.query(seiri_codex::CodexNativeV3QueryKind::Linter);
    let seiri_codex::CodexNativeV3Query::Linter(Some(data)) = linter.query else {
        panic!("linter query");
    };
    assert!(std::ptr::eq(data, &lint));

    let remote = view.query(seiri_codex::CodexNativeV3QueryKind::Remote);
    let seiri_codex::CodexNativeV3Query::Remote(data) = remote.query else {
        panic!("remote query");
    };
    assert!(std::ptr::eq(data, &snapshot.remote_evidence));
    assert_eq!(data.status, RemoteEvidenceStatus::NotRequested);
}

#[test]
fn q34_bound_planner_refuses_stale_analysis_and_keeps_policy_content_unbound() {
    let temp = TempFixture::copy_safe_plan_repo();
    let snapshot = seiri_report::audit_repository_with_profile(&temp.root, ProfileKind::Common)
        .expect("audit fixture");
    let plan = seiri_planner::plan_safe_patches(&snapshot);
    assert_eq!(plan.planner_version, "safe_patch_planner.v4");
    assert!(plan.analysis_run.is_some());
    let operation = plan.operations.first().expect("bound safe operation");
    let binding = operation.binding.as_ref().expect("bound operation");
    let current = fs::read(temp.root.join("README.md")).expect("current README");
    assert_eq!(
        binding
            .preflight_against(&operation.proposal, &current)
            .decision,
        PatchProposalDecision::Ready
    );

    let mut wrong_proposal = operation.proposal.clone();
    wrong_proposal.id.push_str("-changed");
    assert!(binding
        .preflight_against(&wrong_proposal, &current)
        .has_issue(PatchProposalIssueKind::AnalysisBindingMismatch));

    fs::write(
        temp.root.join("README.md"),
        b"# Safe Plan Repo\n\nThe source changed after the audit.\n",
    )
    .expect("change README after audit");
    let stale_plan = seiri_planner::plan_safe_patches(&snapshot);
    assert!(stale_plan.operations.is_empty());
    let blocked = stale_plan
        .blocked
        .iter()
        .find(|item| item.pattern_id == "common.docs.route_present")
        .expect("stale docs route is blocked");
    assert!(blocked.proposal.is_none());
    assert!(blocked.preflight.iter().any(|check| {
        check.kind == PatchPreflightCheckKind::CurrentAnalysisInput
            && check.status == PatchPreflightStatus::Fail
    }));

    let policy_snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("missing-readme-repo"),
        ProfileKind::Library,
    )
    .expect("policy fixture audit");
    let policy_plan = seiri_planner::plan_safe_patches(&policy_snapshot);
    assert!(policy_plan
        .blocked
        .iter()
        .filter(|item| item.gate != seiri_core::GateKind::Safe)
        .all(|item| item.proposal.is_none()));
}
