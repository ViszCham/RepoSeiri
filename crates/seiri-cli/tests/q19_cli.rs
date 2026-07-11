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

#[test]
fn block_y_exposes_all_native_v3_queries_in_json_and_markdown() {
    let root = fixture("safe-plan-repo");
    let queries = [
        "summary",
        "routes",
        "evidence",
        "documents",
        "governance",
        "patches",
        "linter",
        "actions",
        "remote",
    ];

    for query in queries {
        let json = run_codex_format(
            &root,
            "json",
            &["--schema", "native-v3", "--view", "query", "--query", query],
        );
        assert!(json.contains("\"schema_version\": \"seiri.codex.native.v3\""));
        assert!(json.contains(&format!("\"kind\": \"{query}\"")));

        let markdown = run_codex_format(
            &root,
            "markdown",
            &["--schema", "native-v3", "--view", "query", "--query", query],
        );
        assert!(markdown.contains("# RepoSeiri Codex Native v3 Query"));
        assert!(markdown.contains(&format!("- Query: `{query}`")));
    }
}

#[test]
fn block_y_preserves_default_bytes_and_rejects_unsupported_schema_queries() {
    let root = fixture("safe-plan-repo");
    let default = run_codex(&root, &[]);
    let explicit = run_codex(
        &root,
        &[
            "--schema",
            "compatibility-v1",
            "--view",
            "context",
            "--query",
            "summary",
        ],
    );
    assert_eq!(default, explicit);

    let output = codex_command(
        &root,
        "json",
        &[
            "--schema",
            "native-v2",
            "--view",
            "query",
            "--query",
            "evidence",
        ],
    )
    .output()
    .expect("run unsupported seiri codex request");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("CLI stderr UTF-8");
    assert!(stderr.contains("unsupported Codex request"));
    assert!(stderr.contains("schema `native-v2`"));
    assert!(stderr.contains("query `evidence`"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("\"kind\": \"summary\""));
}

#[test]
fn block_x_cli_accepts_explicit_local_priors_without_echoing_private_paths() {
    let private_path = "C:/BLOCK_X_PRIVATE_PATH_SENTINEL/missing-priors.json";
    let output = Command::new(env!("CARGO_BIN_EXE_seiri"))
        .arg("audit")
        .arg("--path")
        .arg(fixture("safe-plan-repo"))
        .arg("--calibration-priors")
        .arg(private_path)
        .output()
        .expect("run seiri audit with missing local prior");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("CLI stderr UTF-8");
    assert!(stderr.contains("failed to read local calibration pack"));
    assert!(!stderr.contains(private_path));
    assert!(!stderr.contains("BLOCK_X_PRIVATE_PATH_SENTINEL"));
}

fn run_codex(root: &Path, extra: &[&str]) -> String {
    run_codex_format(root, "json", extra)
}

fn run_codex_format(root: &Path, format: &str, extra: &[&str]) -> String {
    let output = codex_command(root, format, extra)
        .output()
        .expect("run seiri codex");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("CLI UTF-8")
}

fn codex_command(root: &Path, format: &str, extra: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_seiri"));
    command
        .arg("codex")
        .arg("--path")
        .arg(root)
        .arg("--profile")
        .arg("common")
        .arg("--format")
        .arg(format)
        .args(extra);
    command
}
