use crate::audit_repository_with_options;
use seiri_core::ProfileKind;
use seiri_patterns::{
    ExecutablePatternPack, FixtureExecutionResult, FixtureExecutionStatus, FixtureSuiteReport,
    EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION,
};

mod comparator;

use comparator::evaluate_fixture;

#[must_use]
pub fn run_executable_pattern_pack(pack: &ExecutablePatternPack) -> FixtureSuiteReport {
    let results = pack
        .fixtures()
        .iter()
        .map(|fixture| {
            let Some(root) = pack.fixture_root(&fixture.id) else {
                return FixtureExecutionResult {
                    fixture_id: fixture.id.clone(),
                    kind: fixture.kind,
                    status: FixtureExecutionStatus::AuditError,
                    expectations: Vec::new(),
                };
            };
            let fs_options = seiri_fs::ScanOptions {
                max_depth: fixture.scan_budget.max_depth,
                max_entries: fixture.scan_budget.max_entries,
                ..seiri_fs::ScanOptions::default()
            };
            let snapshot = audit_repository_with_options(
                root,
                ProfileKind::Common,
                &fs_options,
                &seiri_markdown::DocumentIndexOptions::default(),
            );
            match snapshot {
                Ok(snapshot) => evaluate_fixture(pack, fixture, &snapshot),
                Err(_) => FixtureExecutionResult {
                    fixture_id: fixture.id.clone(),
                    kind: fixture.kind,
                    status: FixtureExecutionStatus::AuditError,
                    expectations: Vec::new(),
                },
            }
        })
        .collect();
    FixtureSuiteReport {
        schema_version: EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION,
        pack_fingerprint: pack.fingerprint().to_string(),
        results,
        subprocesses_started: 0,
        network_requests_started: 0,
    }
}
