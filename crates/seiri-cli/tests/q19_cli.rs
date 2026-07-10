use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn q19_cli_keeps_v1_default_and_routes_native_query_and_linter_views() {
    let root = fixture("safe-plan-repo");
    let default = run_codex(&root, &[]);
    let native = run_codex(&root, &["--schema", "native-v2"]);
    let native_v3 = run_codex(
        &root,
        &[
            "--schema",
            "native-v3",
            "--view",
            "query",
            "--query",
            "patches",
        ],
    );
    let query = run_codex(&root, &["--view", "query", "--query", "routes"]);
    let linter = run_codex(&root, &["--view", "linter"]);

    assert!(default.contains("\"schema_version\": \"seiri.block_p.v1\""));
    assert!(native.contains("\"schema_version\": \"seiri.codex.native.v2\""));
    assert!(native_v3.contains("\"schema_version\": \"seiri.codex.native.v3\""));
    assert!(native_v3.contains("\"analysis_run\""));
    assert!(native_v3.contains("\"operation_bindings\""));
    assert!(query.contains("\"schema_version\": \"seiri.codex.query.v2\""));
    assert!(query.contains("\"kind\": \"routes\""));
    assert!(linter.contains("\"schema_version\": \"seiri.codex.linter_context.v2\""));
}

fn run_codex(root: &Path, extra: &[&str]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_seiri"));
    command
        .arg("codex")
        .arg("--path")
        .arg(root)
        .arg("--profile")
        .arg("common")
        .arg("--format")
        .arg("json")
        .args(extra);
    let output = command.output().expect("run seiri codex");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("CLI UTF-8")
}
