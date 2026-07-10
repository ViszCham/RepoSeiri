use seiri_core::{
    BenchmarkActivity, BenchmarkDataset, BenchmarkRepoRecord, CalibrationReviewStatus,
    CalibrationScale, CalibrationSource, CalibrationSourceKind, CalibrationSourceVisibility,
    CoverageScope, CoverageStatus, ObservedPattern, PatternGroup, ProfileKind,
    RemoteEvidenceStatus, RemoteUnavailableReason,
};
use seiri_remote::{
    collect_repository_evidence, RemoteEvidenceOptions, RemoteReadAuthorization, RemoteTransport,
    RemoteTransportError, RemoteTransportResponse,
};
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn q31_remote_adapter_is_opt_in_read_only_and_never_serializes_tokens() {
    let transport = FakeTransport::success(
        r#"{"default_branch":"main","archived":false,"license":{"spdx_id":"MIT"},"html_url":"https://example.invalid/acme/demo"}"#,
    );
    let not_requested =
        collect_repository_evidence(&RemoteEvidenceOptions::not_requested(), &transport);
    assert_eq!(not_requested.status, RemoteEvidenceStatus::NotRequested);
    assert_eq!(transport.calls.get(), 0);

    let authorization = RemoteReadAuthorization::new("remote-token-must-not-serialize")
        .expect("non-empty opaque authorization");
    let options = RemoteEvidenceOptions::github_repository("acme", "demo", Some(authorization))
        .expect("read-only GitHub request");
    let observed = collect_repository_evidence(&options, &transport);
    assert_eq!(observed.status, RemoteEvidenceStatus::Observed);
    assert_eq!(
        observed
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.default_branch.as_deref()),
        Some("main")
    );
    assert!(!serde_json::to_string(&observed)
        .expect("remote report JSON")
        .contains("remote-token-must-not-serialize"));
    assert_eq!(transport.calls.get(), 1);
}

#[test]
fn q31_remote_adapter_keeps_terminal_states_distinct_and_off_the_compatibility_wire() {
    let denied = collect_repository_evidence(
        &remote_options(),
        &FakeTransport::response(RemoteTransportResponse {
            status: 403,
            retry_after_seconds: None,
            rate_limited: false,
            body: Vec::new(),
        }),
    );
    assert_eq!(denied.status, RemoteEvidenceStatus::Denied);

    let not_found = collect_repository_evidence(
        &remote_options(),
        &FakeTransport::response(RemoteTransportResponse {
            status: 404,
            retry_after_seconds: None,
            rate_limited: false,
            body: Vec::new(),
        }),
    );
    assert_eq!(not_found.status, RemoteEvidenceStatus::NotFound);
    assert_eq!(not_found.coverage, CoverageStatus::Complete);

    let rate_limited = collect_repository_evidence(
        &remote_options(),
        &FakeTransport::response(RemoteTransportResponse {
            status: 429,
            retry_after_seconds: Some(60),
            rate_limited: true,
            body: Vec::new(),
        }),
    );
    assert_eq!(
        rate_limited.status,
        RemoteEvidenceStatus::RateLimited {
            retry_after_seconds: Some(60)
        }
    );

    let unavailable = collect_repository_evidence(&remote_options(), &FakeTransport::unavailable());
    assert_eq!(
        unavailable.status,
        RemoteEvidenceStatus::Unavailable(RemoteUnavailableReason::Transport)
    );

    let repo = TempRepo::new("remote-snapshot");
    repo.write("README.md", "# Fixture\n");
    let snapshot = seiri_report::audit_repository_with_remote(
        repo.path(),
        ProfileKind::Common,
        &remote_options(),
        &FakeTransport::success("{}"),
    )
    .expect("remote audit");
    assert_eq!(
        snapshot.remote_evidence.status,
        RemoteEvidenceStatus::Observed
    );
    assert_eq!(
        snapshot
            .coverage
            .record(CoverageScope::RemoteMetadata)
            .map(|record| record.status),
        Some(CoverageStatus::Complete)
    );
    let json = seiri_report::to_json(&snapshot).expect("compatibility JSON");
    let wire: serde_json::Value = serde_json::from_str(&json).expect("wire JSON");
    assert!(wire.get("remote_evidence").is_none());
}

#[test]
fn q32_pattern_pack_requires_all_fixture_classes_and_uses_a_conditional_denominator() {
    let common = seiri_patterns::common_pattern_pack();
    assert_eq!(
        common.fingerprint(),
        seiri_patterns::common_pattern_pack().fingerprint()
    );
    for group in PatternGroup::ALL {
        for kind in seiri_patterns::PatternFixtureKind::ALL {
            assert!(common
                .fixtures()
                .iter()
                .any(|fixture| fixture.group == group && fixture.kind == kind));
        }
    }

    let dataset = dataset_fixture();
    let cli_pack = seiri_patterns::profile_pattern_pack(ProfileKind::Cli);
    let run = seiri_calibration::calibrate_local_dataset_with_pattern_pack(&dataset, &cli_pack);
    let metadata = run.pattern_pack.as_ref().expect("pattern pack metadata");
    assert_eq!(metadata.condition, "profile:cli");
    assert_eq!(metadata.eligible_records, 1);
    assert_eq!(metadata.excluded_records, 1);
    assert_eq!(run.summary.records, 1);
    assert!(metadata.registry_fingerprint.starts_with("fnv1a64:"));
    assert!(run
        .stats
        .iter()
        .all(|stat| stat.repositories <= metadata.eligible_records));
    assert!(run
        .sources
        .iter()
        .all(|source| source.visibility == CalibrationSourceVisibility::LocalOnly));

    let public_json = seiri_report::calibration_to_json(&run).expect("public calibration JSON");
    assert!(!public_json.contains("C:/private-calibration-input"));
    assert!(public_json.contains("redacted-calibration-dataset"));
    assert!(public_json.contains(&metadata.registry_fingerprint));
}

fn remote_options() -> RemoteEvidenceOptions {
    RemoteEvidenceOptions::github_repository("acme", "demo", None)
        .expect("read-only GitHub request")
}

fn dataset_fixture() -> BenchmarkDataset {
    BenchmarkDataset {
        schema_version: seiri_core::SCHEMA_VERSION.to_string(),
        dataset_id: "private-dataset-id".to_string(),
        name: "C:/private-calibration-input".to_string(),
        collected_at: "2026-07-11".to_string(),
        calibration_sources: vec![CalibrationSource {
            id: "private-source".to_string(),
            kind: CalibrationSourceKind::Fixture,
            visibility: CalibrationSourceVisibility::Public,
            label: "C:/private-calibration-input".to_string(),
            collected_at: "2026-07-11".to_string(),
            records: 2,
            scale: CalibrationScale::Tiny,
            metadata_sources: vec!["private metadata source".to_string()],
            extraction_conditions: vec!["private collection condition".to_string()],
            limitations: vec!["private limitation".to_string()],
            evidence_schema: Some(seiri_calibration::default_evidence_schema()),
            review_status: CalibrationReviewStatus::PendingReview,
        }],
        extraction_conditions: vec!["local fixture".to_string()],
        limitations: vec!["fixture only".to_string()],
        evidence_schema: seiri_calibration::default_evidence_schema(),
        records: vec![
            record("cli", ProfileKind::Cli),
            record("docs", ProfileKind::Docs),
        ],
    }
}

fn record(repo_id: &str, profile: ProfileKind) -> BenchmarkRepoRecord {
    BenchmarkRepoRecord {
        repo_id: repo_id.to_string(),
        owner: None,
        name: repo_id.to_string(),
        url: None,
        stars: 0,
        age_days: 0,
        primary_language: None,
        topics: Vec::new(),
        activity: BenchmarkActivity::default(),
        metadata_source: "local fixture".to_string(),
        profile_hint: Some(profile),
        observed_patterns: vec![ObservedPattern {
            pattern_id: Some("common.docs.route_present".to_string()),
            raw_label: "documentation route".to_string(),
            evidence_kind: None,
            route: None,
            location: None,
            count: 1,
        }],
    }
}

struct FakeTransport {
    response: Result<RemoteTransportResponse, RemoteTransportError>,
    calls: Cell<usize>,
}

impl FakeTransport {
    fn response(response: RemoteTransportResponse) -> Self {
        Self {
            response: Ok(response),
            calls: Cell::new(0),
        }
    }

    fn success(body: &str) -> Self {
        Self::response(RemoteTransportResponse {
            status: 200,
            retry_after_seconds: None,
            rate_limited: false,
            body: body.as_bytes().to_vec(),
        })
    }

    fn unavailable() -> Self {
        Self {
            response: Err(RemoteTransportError::Unavailable),
            calls: Cell::new(0),
        }
    }
}

impl RemoteTransport for FakeTransport {
    fn get_repository_metadata(
        &self,
        _request: &seiri_remote::RemoteReadRequest,
    ) -> Result<RemoteTransportResponse, RemoteTransportError> {
        self.calls.set(self.calls.get() + 1);
        self.response.clone()
    }
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
            "reposeiri-q31-q32-{label}-{}-{nonce}",
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
