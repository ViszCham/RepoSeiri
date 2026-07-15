use seiri_core::{CalibrationPriorState, ProfileKind, RouteKind, RouteTargetRole, TargetRelation};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn related_targets_are_relations_not_conflicts() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("related-route-targets"))
        .expect("audit related targets");

    let security_targets = snapshot
        .route_targets
        .iter()
        .filter(|target| target.route == RouteKind::Security)
        .collect::<Vec<_>>();
    assert!(security_targets
        .iter()
        .any(|target| target.role == RouteTargetRole::Canonical));
    assert!(security_targets
        .iter()
        .any(|target| target.role == RouteTargetRole::Detail));
    assert!(snapshot
        .document_consistency
        .relations
        .iter()
        .any(|relation| relation.route == RouteKind::Security
            && relation.relation == TargetRelation::Refines));
    assert!(snapshot
        .document_consistency
        .conflicts
        .iter()
        .all(|conflict| conflict.route != RouteKind::Security));
}

#[test]
fn two_canonical_targets_remain_a_true_conflict() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("conflicting-canonical-targets"))
        .expect("audit true conflict");
    let conflict = snapshot
        .document_consistency
        .conflicts
        .iter()
        .find(|conflict| conflict.route == RouteKind::Security)
        .expect("security conflict");

    assert_eq!(conflict.relation, TargetRelation::Competes);
    assert_eq!(conflict.left.role, RouteTargetRole::Canonical);
    assert_eq!(conflict.right.role, RouteTargetRole::Canonical);
    assert!(conflict.left.span.is_some());
    assert!(conflict.right.span.is_some());
    assert_ne!(conflict.left.target, conflict.right.target);
}

#[test]
fn reposeiri_self_audit_has_no_false_document_conflicts() {
    let snapshot = seiri_report::audit_repository(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("RepoSeiri self-audit");
    assert!(
        snapshot.document_consistency.conflicts.is_empty(),
        "unexpected self-audit conflicts: {:#?}",
        snapshot.document_consistency.conflicts
    );
}

#[test]
fn standard_audit_has_typed_profile_semantics_without_priors() {
    let snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("readme-route-repo"),
        ProfileKind::Library,
    )
    .expect("standard audit");
    let profile = snapshot.profile.as_ref().expect("profile report");

    assert!(profile.branches.iter().all(|branch| {
        branch.semantics.calibration_prior == CalibrationPriorState::NotRequested
    }));
    let json = serde_json::to_string(profile).expect("canonical profile JSON");
    assert!(json.contains("semantics"));
    assert!(json.contains("calibration_prior"));
    let roundtrip: seiri_core::ProfileReport =
        serde_json::from_str(&json).expect("canonical profile roundtrip");
    assert!(roundtrip.branches.iter().all(|branch| {
        branch.semantics.calibration_prior == CalibrationPriorState::NotRequested
    }));
    assert!(profile
        .branch_summary
        .boundary
        .contains("not a probability"));
}

#[test]
fn explicit_local_prior_never_reaches_public_surfaces() {
    let temp = temporary_pack_path();
    let fingerprint = seiri_patterns::common_pattern_pack()
        .fingerprint()
        .to_string();
    let body = serde_json::json!({
        "schema_version": seiri_calibration::LOCAL_PRIOR_SCHEMA_VERSION,
        "registry_fingerprint": fingerprint,
        "private_note": "PRIVATE_CALIBRATION_BODY_SENTINEL",
        "priors": [
            {
                "key": { "kind": "route_gap", "route": "security" },
                "observed": 777_777,
                "sample_size": 999_999,
                "rank_weight_x100": 37
            },
            {
                "key": { "kind": "profile_branch", "profile": "library" },
                "observed": 666_666,
                "sample_size": 999_999,
                "rank_weight_x100": 29
            }
        ]
    });
    fs::write(&temp, serde_json::to_vec(&body).expect("serialize pack")).expect("write local pack");

    let provider =
        seiri_calibration::load_local_calibration_provider(&temp).expect("load local pack");
    assert!(provider.content_fingerprint().starts_with("sha256:"));
    assert_ne!(
        provider.content_fingerprint(),
        provider.registry_fingerprint()
    );
    let snapshot = seiri_report::audit_repository_with_calibration_provider(
        fixture("missing-readme-repo"),
        ProfileKind::Library,
        &provider,
    )
    .expect("audit with explicit calibration");

    let security = snapshot
        .missing_route_priority
        .priorities
        .iter()
        .find(|priority| priority.route == RouteKind::Security)
        .expect("security priority");
    assert_eq!(security.calibration_estimate, None);
    assert!(security.reason.contains("values redacted"));
    let library = snapshot
        .profile
        .as_ref()
        .expect("profile")
        .branches
        .iter()
        .find(|branch| branch.profile == ProfileKind::Library)
        .expect("library branch");
    assert_eq!(
        library.semantics.calibration_prior,
        CalibrationPriorState::AppliedRedacted
    );

    let plan = seiri_planner::plan_patches(&snapshot);
    let codex = seiri_codex::CodexView::new(&snapshot, &plan, None);
    let context = codex.query(seiri_codex::CodexQueryKind::Summary);
    let surfaces = [
        seiri_report::to_json(&snapshot).expect("snapshot JSON"),
        seiri_report::to_markdown(&snapshot),
        serde_json::to_string(&context).expect("Codex JSON"),
        seiri_codex::render_query_markdown(&context),
        format!("{snapshot:?}"),
    ];
    let path_text = temp.to_string_lossy().to_string();
    for surface in surfaces {
        for forbidden in [
            "PRIVATE_CALIBRATION_BODY_SENTINEL",
            "777777",
            "999999",
            "666666",
            path_text.as_str(),
        ] {
            assert!(
                !surface.contains(forbidden),
                "local prior detail leaked into a public surface"
            );
        }
    }

    fs::remove_file(&temp).expect("remove local pack");
    fs::remove_dir(temp.parent().expect("temporary parent")).expect("remove temporary directory");
}

#[test]
fn local_prior_parse_errors_redact_path_and_invalid_values() {
    let temp = temporary_pack_path();
    let fingerprint = seiri_patterns::common_pattern_pack()
        .fingerprint()
        .to_string();
    let private_value = "PRIVATE_CALIBRATION_INVALID_ROUTE_SENTINEL";
    let body = serde_json::json!({
        "schema_version": seiri_calibration::LOCAL_PRIOR_SCHEMA_VERSION,
        "registry_fingerprint": fingerprint,
        "priors": [{
            "key": { "kind": "route_gap", "route": private_value },
            "observed": 1,
            "sample_size": 1,
            "rank_weight_x100": 1
        }]
    });
    fs::write(
        &temp,
        serde_json::to_vec(&body).expect("serialize invalid pack"),
    )
    .expect("write invalid local pack");

    let error = seiri_calibration::load_local_calibration_provider(&temp)
        .err()
        .expect("invalid route must fail");
    let path_text = temp.to_string_lossy();
    for surface in [error.to_string(), format!("{error:?}")] {
        assert!(!surface.contains(private_value));
        assert!(!surface.contains(path_text.as_ref()));
    }

    fs::remove_file(&temp).expect("remove invalid local pack");
    fs::remove_dir(temp.parent().expect("temporary parent")).expect("remove temporary directory");
}

#[test]
fn local_prior_loader_rejects_invalid_counts_fingerprint_and_duplicates() {
    use seiri_calibration::LocalPriorLoadError;

    let temp = temporary_pack_path();
    let fingerprint = seiri_patterns::common_pattern_pack()
        .fingerprint()
        .to_string();
    let key = serde_json::json!({ "kind": "route_gap", "route": "security" });

    write_pack(
        &temp,
        &fingerprint,
        vec![serde_json::json!({
            "key": key.clone(),
            "observed": 1,
            "sample_size": 0,
            "rank_weight_x100": 1
        })],
    );
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider(&temp),
        Err(LocalPriorLoadError::InvalidPrior)
    ));

    write_pack(
        &temp,
        &fingerprint,
        vec![serde_json::json!({
            "key": key.clone(),
            "observed": 2,
            "sample_size": 1,
            "rank_weight_x100": 1
        })],
    );
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider(&temp),
        Err(LocalPriorLoadError::InvalidPrior)
    ));

    write_pack(
        &temp,
        &fingerprint,
        vec![serde_json::json!({
            "key": key.clone(),
            "observed": 1,
            "sample_size": 1,
            "rank_weight_x100": 101
        })],
    );
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider(&temp),
        Err(LocalPriorLoadError::InvalidPrior)
    ));

    write_pack(
        &temp,
        "fnv1a64:0000000000000000",
        vec![serde_json::json!({
            "key": key.clone(),
            "observed": 1,
            "sample_size": 1,
            "rank_weight_x100": 1
        })],
    );
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider(&temp),
        Err(LocalPriorLoadError::RegistryFingerprintMismatch)
    ));

    let duplicate = serde_json::json!({
        "key": key,
        "observed": 1,
        "sample_size": 1,
        "rank_weight_x100": 1
    });
    write_pack(&temp, &fingerprint, vec![duplicate.clone(), duplicate]);
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider(&temp),
        Err(LocalPriorLoadError::DuplicateKey)
    ));

    fs::write(&temp, vec![b' '; 2 * 1024 * 1024 + 1]).expect("write oversized pack");
    assert!(matches!(
        seiri_calibration::load_local_calibration_provider(&temp),
        Err(LocalPriorLoadError::SourceTooLarge)
    ));

    fs::remove_file(&temp).expect("remove validation pack");
    fs::remove_dir(temp.parent().expect("temporary parent")).expect("remove temporary directory");
}

fn write_pack(path: &Path, fingerprint: &str, priors: Vec<serde_json::Value>) {
    let body = serde_json::json!({
        "schema_version": seiri_calibration::LOCAL_PRIOR_SCHEMA_VERSION,
        "registry_fingerprint": fingerprint,
        "priors": priors
    });
    fs::write(path, serde_json::to_vec(&body).expect("serialize pack"))
        .expect("write validation pack");
}

fn temporary_pack_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "reposeiri-private-calibration-sentinel-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("create temporary directory");
    directory.join("private-calibration-values.json")
}
