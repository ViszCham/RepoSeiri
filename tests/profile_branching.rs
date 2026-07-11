use seiri_core::{
    BaselineStatus, CalibrationPriorState, ProfileEvidenceBasis, ProfileKind, ProfileWeightBasis,
};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn profile_rules_change_recommendation_order_by_repository_purpose() {
    let cli_snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("docs-routed-repo"),
        ProfileKind::Cli,
    )
    .expect("audit cli profile");
    let infra_snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("docs-routed-repo"),
        ProfileKind::Infra,
    )
    .expect("audit infra profile");

    let cli_profile = cli_snapshot.profile.as_ref().expect("cli profile");
    let infra_profile = infra_snapshot.profile.as_ref().expect("infra profile");

    assert_eq!(
        cli_profile.recommendations[0].pattern_id,
        "common.quickstart.route_present"
    );
    assert_eq!(
        infra_profile.recommendations[0].pattern_id,
        "common.security.route_present"
    );
    assert_ne!(
        cli_profile.recommendations[0].pattern_id,
        infra_profile.recommendations[0].pattern_id
    );
}

#[test]
fn profile_score_view_is_deterministic_and_bounded() {
    let snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("docs-routed-repo"),
        ProfileKind::Docs,
    )
    .expect("audit docs profile");
    let profile = snapshot.profile.expect("profile report");

    assert_eq!(profile.profile, ProfileKind::Docs);
    assert!(profile.score.score_x100 <= 100);
    assert!(profile.score.total_weight > 0);
    assert!(profile.score.earned_weight <= profile.score.total_weight);
    assert_eq!(
        profile.score.evidence_basis,
        ProfileEvidenceBasis::RepositoryEvidence
    );
    assert_eq!(
        profile.score.weight_basis,
        ProfileWeightBasis::StaticProfileRegistry
    );
    assert!(profile
        .rules
        .iter()
        .any(|rule| rule.status == BaselineStatus::Present));
    assert!(profile
        .rules
        .iter()
        .any(|rule| rule.status == BaselineStatus::Missing));
    assert!(profile.score.note.contains("Calibration estimates remain"));
    assert_eq!(profile.branch_summary.selected_profile, ProfileKind::Docs);
    assert_eq!(profile.branches.len(), 9);
    assert!(profile.branch_summary.top_profile.is_some());
    assert!(profile.branch_summary.top_rank_score_x100.is_some());
}

#[test]
fn profile_report_keeps_missing_recommendations_ranked() {
    let snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("missing-readme-repo"),
        ProfileKind::Library,
    )
    .expect("audit library profile");
    let profile = snapshot.profile.expect("profile report");

    assert!(!profile.recommendations.is_empty());
    for (index, recommendation) in profile.recommendations.iter().enumerate() {
        assert_eq!(recommendation.rank, index + 1);
        assert!(recommendation.weight > 0);
    }
}

#[test]
fn profile_branch_semantics_emit_multiple_candidates_without_implicit_priors() {
    let snapshot = seiri_report::audit_repository_subtree_with_profile(
        fixture("readme-route-repo"),
        ProfileKind::Common,
    )
    .expect("audit common profile");
    let profile = snapshot.profile.expect("profile report");

    assert_eq!(profile.branch_summary.selected_profile, ProfileKind::Common);
    assert_eq!(profile.branch_summary.emitted_profiles, 9);
    assert_eq!(profile.branches.len(), 9);
    assert!(profile
        .branch_summary
        .boundary
        .contains("not a repository type assertion"));
    assert!(profile.branches.windows(2).all(|window| {
        window[0].semantics.rank_score.get() >= window[1].semantics.rank_score.get()
    }));

    let library = profile
        .branches
        .iter()
        .find(|branch| branch.profile == ProfileKind::Library)
        .expect("library branch");
    let cli = profile
        .branches
        .iter()
        .find(|branch| branch.profile == ProfileKind::Cli)
        .expect("cli branch");
    let product = profile
        .branches
        .iter()
        .find(|branch| branch.profile == ProfileKind::Product)
        .expect("product branch");
    let runtime = profile
        .branches
        .iter()
        .find(|branch| branch.profile == ProfileKind::Runtime)
        .expect("runtime branch");
    let ml = profile
        .branches
        .iter()
        .find(|branch| branch.profile == ProfileKind::Ml)
        .expect("ml branch");

    for branch in [library, cli, product, runtime, ml] {
        assert_eq!(
            branch.semantics.calibration_prior,
            CalibrationPriorState::NotRequested
        );
    }
    assert!(library.semantics.rank_score.get() > 0);
    assert!(cli.semantics.rank_score.get() > 0);
    assert!(!library.matched_signals.is_empty());
}
