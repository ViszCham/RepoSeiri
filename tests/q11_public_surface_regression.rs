use seiri_core::{CalibrationSourceVisibility, ProfileKind};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn assert_no_local_only_details(output: &str) {
    for token in [
        "SYNTHETIC_LOCAL_ONLY_DATASET_ID_SHOULD_NOT_RENDER",
        "SYNTHETIC_LOCAL_ONLY_PATH_SHOULD_NOT_RENDER",
        "SYNTHETIC_LOCAL_ONLY_BODY_SHOULD_NOT_RENDER",
    ] {
        assert!(
            !output.contains(token),
            "public output leaked synthetic local-only token `{token}`"
        );
    }
}

#[test]
fn q11_local_only_calibration_redaction_and_codex_no_github_mutation_are_fixed() {
    let dataset = seiri_calibration::load_dataset(fixture("calibration-local-only-dataset.json"))
        .expect("load local-only calibration fixture");
    let run = seiri_calibration::calibrate_dataset(&dataset);
    assert_eq!(
        run.sources[1].visibility,
        CalibrationSourceVisibility::LocalOnly
    );

    let json = seiri_report::calibration_to_json(&run).expect("render calibration JSON");
    let markdown = seiri_report::calibration_to_markdown(&run);
    let codex_summary = seiri_codex::build_calibration_source_summary(&run);
    let codex_json = serde_json::to_string(&codex_summary).expect("render codex summary");

    assert!(json.contains("\"visibility\": \"redacted\""));
    assert!(markdown.contains("Source visibility: public `1` / local_only `1` / redacted `0`"));
    assert_no_local_only_details(&json);
    assert_no_local_only_details(&markdown);
    assert_no_local_only_details(&codex_json);

    let context =
        seiri_report::codex_repository_with_profile(fixture("safe-plan-repo"), ProfileKind::Common)
            .expect("codex context");
    assert!(!context.user_actions.is_empty());
    assert!(context
        .user_actions
        .iter()
        .all(|action| !action.mutates_files && !action.requires_confirmation));

    let codex_markdown = seiri_report::codex_to_markdown(&context);
    let pr_body = seiri_report::codex_pr_body_to_markdown(&context);
    let codex_json = seiri_report::codex_to_json(&context).expect("codex JSON");
    for surface in [&codex_markdown, &pr_body, &codex_json] {
        for forbidden in [
            "git push",
            "git checkout -b",
            "git switch -c",
            "gh pr create",
            "api.github.com",
        ] {
            assert!(
                !surface.contains(forbidden),
                "Codex context contained GitHub mutation token `{forbidden}`"
            );
        }
    }
    assert!(pr_body.contains("RepoSeiri did not create this PR, push a branch, call GitHub"));
}

#[test]
fn q11_reposeiri_self_audit_smoke_keeps_public_surfaces_available() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshot = seiri_report::audit_repository_with_profile(root, ProfileKind::Library)
        .expect("self audit should run");
    assert!(!snapshot.files.is_empty());
    assert!(!snapshot.evidence_ledger.is_empty());
    assert!(!snapshot.route_states.is_empty());
    assert!(!snapshot.claims.is_empty());

    let audit_markdown = seiri_report::to_markdown(&snapshot);
    assert!(audit_markdown.contains("# RepoSeiri Report"));
    assert!(audit_markdown.contains("## Content Claims"));

    let context = seiri_report::codex_repository_with_profile(root, ProfileKind::Library)
        .expect("self codex context should run");
    assert!(context.claims.total > 0);
    assert!(context.wording_lint.available);
    assert!(!context.route_meanings.is_empty());

    let codex_json = seiri_report::codex_to_json(&context).expect("render codex JSON");
    assert!(codex_json.contains("\"claims\""));
    assert!(codex_json.contains("\"wording_lint\""));
    assert!(codex_json.contains("\"route_meanings\""));
}
