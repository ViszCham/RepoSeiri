use seiri_core::{
    ByteOffset, CoverageIncompleteReason, CoverageIndex, CoverageScope, CoverageStatus, DocumentId,
    Observation, ReviewGap, RouteKind, RouteState, SourceDomain, UnknownReason,
};
use std::mem::size_of;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn route_and_content_gaps_are_distinct() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    let security = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::Security)
        .expect("security assessment");
    assert_eq!(security.summary_projection().state, RouteState::Verified);
    assert_ne!(
        snapshot.missing_route_priority.summary.top_route,
        Some(RouteKind::Security),
        "a verified content gap must not become the top missing route"
    );

    let security_content = snapshot
        .review_priority
        .priorities
        .iter()
        .find(|priority| {
            matches!(
                &priority.gap,
                ReviewGap::Content {
                    route: RouteKind::Security,
                    candidate_pattern_ids,
                } if candidate_pattern_ids.iter().any(|id| id == "SEC-004")
            )
        })
        .expect("security content gap");
    assert_eq!(security_content.gap.route(), Some(RouteKind::Security));
    assert!(snapshot.review_priority.summary.content_gaps > 0);
}

#[test]
fn nested_license_is_local_not_inherited() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("nested-license-only-repo"))
        .expect("audit fixture");
    let license = snapshot
        .route_assessments
        .iter()
        .find(|assessment| assessment.route() == RouteKind::License)
        .expect("license assessment");

    assert!(!license.presence().inherited());
    assert!(license.evidence().inherited().is_empty());
    assert!(snapshot
        .evidence_kernel
        .facts()
        .iter()
        .all(|fact| fact.provenance.domain != SourceDomain::OrganizationInherited));
}

#[test]
fn typed_kernel_deduplicates_document_paths_and_is_in_canonical_wire() {
    let snapshot = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit fixture");
    assert_eq!(
        snapshot.evidence_kernel.facts().len(),
        snapshot.evidence_kernel.facts().len()
    );
    assert!(snapshot
        .evidence_kernel
        .documents()
        .windows(2)
        .all(|pair| pair[0].path < pair[1].path));
    assert_eq!(size_of::<DocumentId>(), size_of::<u32>());
    assert_eq!(size_of::<ByteOffset>(), size_of::<u32>());

    let wire = seiri_report::to_json(&snapshot).expect("canonical JSON");
    assert!(wire.contains("evidence_kernel"));
    assert!(wire.contains("review_priority"));
    assert!(wire.contains("\"coverage\""));
}

#[test]
fn absence_requires_complete_coverage() {
    let coverage = CoverageIndex::try_new([
        (CoverageScope::RepositoryFiles, CoverageStatus::Complete),
        (
            CoverageScope::RemoteMetadata,
            CoverageStatus::Partial(CoverageIncompleteReason::LimitExceeded),
        ),
    ])
    .expect("coverage index");

    assert!(matches!(
        coverage.observe_absence::<()>(CoverageScope::RepositoryFiles),
        Observation::Absent { .. }
    ));
    assert_eq!(
        coverage.observe_absence::<()>(CoverageScope::RemoteMetadata),
        Observation::Unknown(UnknownReason::LimitExceeded)
    );
    assert_eq!(
        CoverageIndex::default().observe_absence::<()>(CoverageScope::RootReadme),
        Observation::Unknown(UnknownReason::NotRequested)
    );
    assert!(CoverageIndex::try_new([
        (CoverageScope::RootReadme, CoverageStatus::Complete),
        (CoverageScope::RootReadme, CoverageStatus::Complete),
    ])
    .is_err());
    assert!(Observation::present((), Vec::new()).is_err());
}
