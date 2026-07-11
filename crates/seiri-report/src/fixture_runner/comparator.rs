use seiri_core::{Observation, PatternOutcome, RepositoryAnalysis};
use seiri_patterns::{
    EvidenceExpectation, ExecutablePatternPack, FixtureExecutionResult, FixtureExecutionStatus,
    FixtureExpectation, FixtureExpectationActual, FixtureExpectationResult, PredicateContext,
};

pub(super) fn evaluate_fixture(
    pack: &ExecutablePatternPack,
    fixture: &seiri_patterns::ExecutableFixtureSpec,
    snapshot: &RepositoryAnalysis,
) -> FixtureExecutionResult {
    let expectations = fixture
        .expectations
        .iter()
        .enumerate()
        .map(|(index, expectation)| evaluate_expectation(pack, snapshot, index, expectation))
        .collect::<Vec<_>>();
    let status = if expectations.iter().all(|result| result.passed) {
        FixtureExecutionStatus::Passed
    } else {
        FixtureExecutionStatus::Failed
    };
    FixtureExecutionResult {
        fixture_id: fixture.id.clone(),
        kind: fixture.kind,
        status,
        expectations,
    }
}

fn evaluate_expectation(
    pack: &ExecutablePatternPack,
    snapshot: &RepositoryAnalysis,
    index: usize,
    expectation: &FixtureExpectation,
) -> FixtureExpectationResult {
    let (passed, actual) = match expectation {
        FixtureExpectation::Pattern {
            pattern,
            outcome,
            evidence,
        } => {
            let observation = pack.definition(pattern).map(|definition| {
                definition
                    .predicate
                    .evaluate(PredicateContext::from_snapshot(snapshot))
            });
            let (actual_outcome, evidence_ids) = match observation {
                Some(Observation::Present { evidence, .. }) => {
                    (Some(PatternOutcome::Present), evidence.as_slice().to_vec())
                }
                Some(Observation::Absent { .. }) => (Some(PatternOutcome::Missing), Vec::new()),
                Some(Observation::Unknown(_) | Observation::Conflict { .. }) | None => {
                    (None, Vec::new())
                }
            };
            let evidence_matches = match evidence {
                EvidenceExpectation::Any => true,
                EvidenceExpectation::AtLeast(minimum) => {
                    evidence_ids.len() >= usize::from(*minimum)
                }
                EvidenceExpectation::Exact(expected) => evidence_ids == *expected,
            };
            (
                actual_outcome == Some(*outcome) && evidence_matches,
                FixtureExpectationActual::Pattern {
                    outcome: actual_outcome,
                    evidence_ids,
                },
            )
        }
        FixtureExpectation::Coverage { scope, status } => {
            let actual_status = snapshot.coverage.record(*scope).map(|record| record.status);
            (
                actual_status == Some(*status),
                FixtureExpectationActual::Coverage {
                    status: actual_status,
                },
            )
        }
        FixtureExpectation::Gap {
            gap,
            minimum,
            maximum,
        } => {
            let count = snapshot
                .review_priority
                .priorities
                .iter()
                .filter(|priority| priority.gap.kind() == *gap)
                .count();
            (
                count >= usize::from(*minimum) && count <= usize::from(*maximum),
                FixtureExpectationActual::Count { value: count },
            )
        }
        FixtureExpectation::ClaimBoundary { boundary, present } => {
            let actual_present = snapshot
                .claims
                .iter()
                .any(|claim| claim.boundaries.contains(boundary));
            (
                actual_present == *present,
                FixtureExpectationActual::Boundary {
                    present: actual_present,
                },
            )
        }
        FixtureExpectation::Diagnostic { minimum } => {
            let markdown = snapshot
                .document_index
                .scanned_documents()
                .filter_map(|document| document.scan.as_ref())
                .map(|document| document.diagnostics().len())
                .sum::<usize>();
            let github = snapshot
                .github_local_documents
                .documents()
                .iter()
                .map(|document| document.diagnostics.len())
                .sum::<usize>();
            let count = markdown + github;
            (
                count >= usize::from(*minimum),
                FixtureExpectationActual::Count { value: count },
            )
        }
    };
    FixtureExpectationResult {
        index,
        passed,
        actual,
    }
}
