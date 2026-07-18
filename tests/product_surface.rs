use seiri_codex::{CodexQueryKind, CodexView};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    repository_root().join("fixtures").join(name)
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repository_root().join(path)).expect("read product surface")
}

#[test]
fn readme_example_is_generated_from_the_public_fixture() {
    let analysis = seiri_report::audit_repository_subtree(fixture("readme-route-repo"))
        .expect("audit public README fixture");
    let plan = seiri_planner::plan_patches(&analysis);
    let adapter = CodexView::new(&analysis, &plan, None);
    let view = adapter.query(CodexQueryKind::Summary);
    let rendered = seiri_codex::render_query_markdown(&view);
    let expected = include_str!("snapshots/readme-route-summary.md");

    assert_eq!(rendered.trim_end(), expected.trim_end());
    let readme = read("README.md");
    assert_eq!(
        readme.matches(expected.trim_end()).count(),
        2,
        "Japanese and English examples must both use the fixture snapshot"
    );
    assert!(!readme.contains("<count>"));
    assert!(!readme.contains("<path>"));
}

#[test]
fn readme_cli_plugin_and_release_surfaces_share_the_v1_contract() {
    let readme = read("README.md");
    let cli = read("crates/seiri-cli/src/main.rs");
    let skill = read("plugins/reposeiri/skills/reposeiri/SKILL.md");
    let migration = read("docs/migration-v3.md");
    let release = read("docs/release.md");
    let changelog = read("CHANGELOG.md");
    let cargo = read("Cargo.toml");
    let plugin: serde_json::Value =
        serde_json::from_str(&read("plugins/reposeiri/.codex-plugin/plugin.json"))
            .expect("plugin manifest JSON");

    assert!(cargo.contains("version = \"1.0.0\""));
    assert_eq!(plugin["version"], "1.0.0");
    for surface in [&readme, &skill, &release, &changelog] {
        assert!(surface.contains("1.0.0"));
    }

    for query in CodexQueryKind::ALL {
        let slug = query.slug();
        for surface in [&readme, &cli, &skill] {
            assert!(
                surface.contains(slug),
                "public product surface omitted query {slug}"
            );
        }
    }

    for wire in [
        "seiri.analysis.v2",
        "seiri.patch-plan.v2",
        "seiri.codex.v2",
        "seiri.completion.v3",
        "reposeiri.runtime-manifest.v3",
    ] {
        assert!(
            migration.contains(wire) || release.contains(wire),
            "migration/release docs omitted {wire}"
        );
    }

    let japanese = readme.find("## 日本語").expect("Japanese section");
    let english = readme.find("## English").expect("English section");
    assert!(japanese < english);
    assert_eq!(readme.matches("cargo test --workspace --locked").count(), 2);
    assert_eq!(readme.matches("fixtures/readme-route-repo").count(), 4);
}

#[test]
fn public_extension_and_calibration_schemas_pin_current_fields() {
    let executable: serde_json::Value =
        serde_json::from_str(&read("schemas/seiri.executable-pattern-pack.v2.json"))
            .expect("executable pattern schema");
    let calibration: serde_json::Value =
        serde_json::from_str(&read("schemas/seiri.calibration.v2.json"))
            .expect("calibration schema");
    let analysis: serde_json::Value =
        serde_json::from_str(&read("schemas/seiri.analysis.v2.json")).expect("analysis schema");
    let holdout: serde_json::Value =
        serde_json::from_str(&read("schemas/seiri.calibration-holdout.v1.json"))
            .expect("holdout report schema");
    let corpus: serde_json::Value =
        serde_json::from_str(&read("schemas/seiri.calibration-corpus.v1.json"))
            .expect("holdout corpus schema");

    assert_eq!(
        executable["$defs"]["definition"]["properties"]["enabled"]["type"],
        "boolean"
    );
    assert_eq!(
        executable["$defs"]["definition"]["properties"]["adoption_stage"]["const"],
        "candidate"
    );
    assert_eq!(
        calibration["$defs"]["support_interval"]["properties"]["method"]["const"],
        "wilson_95"
    );
    assert!(calibration["$defs"]["pattern_stats"]["required"]
        .as_array()
        .expect("pattern stats required")
        .iter()
        .any(|field| field == "local_support_tier"));
    assert!(
        analysis["$defs"]["pattern_extensions"]["properties"]["evaluations"]["items"]["required"]
            .as_array()
            .expect("extension evaluation required")
            .iter()
            .any(|field| field == "state")
    );
    assert_eq!(
        holdout["properties"]["status"]["$ref"],
        "#/$defs/calibration_status"
    );
    assert_eq!(holdout["properties"]["task_metrics"]["minItems"], 5);
    assert_eq!(
        holdout["$defs"]["task_metric"]["properties"]["independent_holdout_cases"]["type"],
        "integer"
    );
    assert_eq!(
        holdout["$defs"]["task_metric"]["properties"]["accuracy_interval"]["$ref"],
        "#/$defs/interval"
    );
    assert_eq!(
        corpus["$defs"]["case"]["properties"]["expectation"]["oneOf"]
            .as_array()
            .expect("typed expectations")
            .len(),
        5
    );
}
