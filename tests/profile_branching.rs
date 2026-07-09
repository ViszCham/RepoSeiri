use seiri_core::{BaselineStatus, ProfileKind};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn profile_rules_change_recommendation_order_by_repository_purpose() {
    let cli_snapshot =
        seiri_report::audit_repository_with_profile(fixture("docs-routed-repo"), ProfileKind::Cli)
            .expect("audit cli profile");
    let infra_snapshot = seiri_report::audit_repository_with_profile(
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
    let snapshot =
        seiri_report::audit_repository_with_profile(fixture("docs-routed-repo"), ProfileKind::Docs)
            .expect("audit docs profile");
    let profile = snapshot.profile.expect("profile report");

    assert_eq!(profile.profile, ProfileKind::Docs);
    assert!(profile.score.score_x100 <= 100);
    assert!(profile.score.total_weight > 0);
    assert!(profile.score.earned_weight <= profile.score.total_weight);
    assert!(profile
        .rules
        .iter()
        .any(|rule| rule.status == BaselineStatus::Present));
    assert!(profile
        .rules
        .iter()
        .any(|rule| rule.status == BaselineStatus::Missing));
    assert!(profile.score.note.contains("not a popularity"));
}

#[test]
fn profile_report_keeps_missing_recommendations_ranked() {
    let snapshot = seiri_report::audit_repository_with_profile(
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
