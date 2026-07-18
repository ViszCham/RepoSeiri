use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn plugin_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins/reposeiri")
}

#[cfg(windows)]
fn launcher_command(binary: &Path) -> Command {
    let mut command = Command::new("powershell");
    command.args([
        "-NoProfile",
        "-File",
        plugin_root()
            .join("scripts/reposeiri-codex.ps1")
            .to_str()
            .expect("launcher path"),
        "-Path",
        ".",
        "-Query",
        "summary",
        "-Format",
        "json",
    ]);
    command.env("REPOSEIRI_BIN", binary);
    command
}

#[cfg(not(windows))]
fn launcher_command(binary: &Path) -> Command {
    let mut command = Command::new("sh");
    command
        .arg(plugin_root().join("scripts/reposeiri-codex.sh"))
        .args(["--path", ".", "--query", "summary", "--format", "json"])
        .env("REPOSEIRI_BIN", binary);
    command
}

#[test]
fn launcher_runs_outside_the_reposeiri_workspace() {
    let unrelated = tempfile::tempdir().expect("temp repository");
    fs::write(unrelated.path().join("README.md"), "# Unrelated\n").expect("README");
    let output = launcher_command(Path::new(env!("CARGO_BIN_EXE_seiri")))
        .current_dir(unrelated.path())
        .output()
        .expect("standalone launcher");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("seiri.codex.v2"));
}

#[test]
fn launcher_rejects_a_missing_configured_binary() {
    let unrelated = tempfile::tempdir().expect("temp repository");
    let output = launcher_command(&unrelated.path().join("missing-seiri"))
        .current_dir(unrelated.path())
        .output()
        .expect("missing binary");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("configured_binary_missing"));
}

#[cfg(windows)]
#[test]
fn launcher_rejects_an_incomplete_contract() {
    let unrelated = tempfile::tempdir().expect("temp repository");
    let fake = unrelated.path().join("seiri.cmd");
    fs::write(
        &fake,
        "@echo off\r\nif \"%1\"==\"contract\" (echo {\"codex_schema\": \"seiri.codex.v2\"}& exit /b 0)\r\nexit /b 17\r\n",
    )
    .expect("fake binary");
    let output = launcher_command(&fake)
        .current_dir(unrelated.path())
        .output()
        .expect("native failure");
    assert_eq!(output.status.code(), Some(5));
    assert!(String::from_utf8_lossy(&output.stderr).contains("schema_mismatch"));
}

#[cfg(not(windows))]
#[test]
fn launcher_rejects_an_incomplete_contract() {
    use std::os::unix::fs::PermissionsExt;
    let unrelated = tempfile::tempdir().expect("temp repository");
    let fake = unrelated.path().join("seiri");
    fs::write(
        &fake,
        "#!/bin/sh\nif [ \"$1\" = contract ]; then printf '{\"codex_schema\": \"seiri.codex.v2\"}'; exit 0; fi\nexit 17\n",
    )
    .expect("fake binary");
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("executable");
    let output = launcher_command(&fake)
        .current_dir(unrelated.path())
        .output()
        .expect("native failure");
    assert_eq!(output.status.code(), Some(5));
    assert!(String::from_utf8_lossy(&output.stderr).contains("schema_mismatch"));
}
