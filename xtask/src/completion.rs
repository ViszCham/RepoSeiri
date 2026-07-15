use crate::{bundle, repository_root};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Instant;

const CHECKS: &[CheckSpec] = &[
    CheckSpec::cargo("format", &["fmt", "--all", "--", "--check"]),
    CheckSpec::cargo("workspace_tests", &["test", "--workspace", "--locked"]),
    CheckSpec::cargo(
        "clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    ),
    CheckSpec::cargo_toolchain(
        "msrv",
        "+1.88.0",
        &["check", "--workspace", "--all-targets", "--locked"],
    ),
    CheckSpec::cargo(
        "schema_contracts",
        &["test", "--test", "completion_contract", "--locked"],
    ),
    CheckSpec::cargo(
        "privacy_boundary",
        &[
            "test",
            "--test",
            "privacy_guard",
            "--test",
            "semantic_privacy",
            "--locked",
        ],
    ),
    CheckSpec::cargo(
        "hostile_corpus",
        &["test", "--test", "hostile_input_corpus", "--locked"],
    ),
    CheckSpec::cargo(
        "plugin_smoke",
        &[
            "test",
            "-p",
            "seiri-cli",
            "--test",
            "standalone_launcher",
            "--locked",
        ],
    ),
    CheckSpec::cargo(
        "self_audit_summary",
        &[
            "run",
            "--locked",
            "--quiet",
            "-p",
            "seiri-cli",
            "--",
            "codex",
            "--path",
            ".",
            "--profile",
            "library",
            "--scope",
            "repository",
            "--query",
            "summary",
            "--format",
            "json",
        ],
    ),
    CheckSpec::cargo(
        "self_audit_linter",
        &[
            "run",
            "--locked",
            "--quiet",
            "-p",
            "seiri-cli",
            "--",
            "codex",
            "--path",
            ".",
            "--profile",
            "library",
            "--scope",
            "repository",
            "--query",
            "linter",
            "--format",
            "json",
        ],
    ),
    CheckSpec::program("diff_hygiene", "git", &["diff", "--check"]),
];

#[derive(Clone, Copy)]
struct CheckSpec {
    name: &'static str,
    program: &'static str,
    toolchain: Option<&'static str>,
    args: &'static [&'static str],
}

impl CheckSpec {
    const fn cargo(name: &'static str, args: &'static [&'static str]) -> Self {
        Self {
            name,
            program: "cargo",
            toolchain: None,
            args,
        }
    }

    const fn cargo_toolchain(
        name: &'static str,
        toolchain: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            program: "cargo",
            toolchain: Some(toolchain),
            args,
        }
    }

    const fn program(
        name: &'static str,
        program: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            program,
            toolchain: None,
            args,
        }
    }
}

#[derive(Debug, Serialize)]
struct CompletionRecord {
    schema_version: &'static str,
    state: CompletionState,
    tool_version: &'static str,
    source: SourceBinding,
    checks: Vec<CheckRecord>,
    required_hosts: Vec<bundle::HostEvidenceRecord>,
    skipped_checks: Vec<String>,
    boundary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CompletionState {
    ReadyForGit,
    Incomplete,
}

#[derive(Debug, Serialize)]
struct CheckRecord {
    name: &'static str,
    status: CheckStatus,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    command: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SourceBinding {
    git_head: String,
    worktree_dirty: bool,
    source_digest: String,
    cargo_lock_digest: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Passed,
    Failed,
    CouldNotStart,
}

pub fn run(args: &[OsString]) -> Result<ExitCode, String> {
    if option(args, "--format")? != "json" {
        return Err("completion supports only '--format json'".to_string());
    }
    let root = repository_root()?;
    let source = bind_source(&root)?;
    let mut checks = CHECKS
        .iter()
        .map(|spec| run_check(&root, *spec))
        .collect::<Vec<_>>();
    let required_hosts =
        bundle::validate_required_hosts(optional_option(args, "--host-evidence").map(Path::new));
    checks.push(CheckRecord {
        name: "required_host_matrix",
        status: if required_hosts
            .iter()
            .all(|host| host.status == bundle::HostEvidenceStatus::Passed)
        {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        exit_code: None,
        elapsed_ms: 0,
        command: vec!["host-evidence".to_string()],
    });
    let state = if checks
        .iter()
        .all(|check| matches!(check.status, CheckStatus::Passed))
    {
        CompletionState::ReadyForGit
    } else {
        CompletionState::Incomplete
    };
    let record = CompletionRecord {
        schema_version: seiri_core::COMPLETION_SCHEMA_VERSION,
        state,
        tool_version: env!("CARGO_PKG_VERSION"),
        source,
        checks,
        required_hosts,
        skipped_checks: Vec::new(),
        boundary: "completion reports one verified worktree state; it does not authorize commit, push, merge, release, plugin installation, restart, or visibility changes",
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&record).map_err(|error| error.to_string())?
    );
    Ok(if state == CompletionState::ReadyForGit {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn run_check(root: &Path, spec: CheckSpec) -> CheckRecord {
    let started = Instant::now();
    let mut command = Command::new(spec.program);
    if let Some(toolchain) = spec.toolchain {
        command.arg(toolchain);
    }
    match command.args(spec.args).current_dir(root).output() {
        Ok(output) => CheckRecord {
            name: spec.name,
            status: if output.status.success()
                && validate_captured_output(spec.name, &output.stdout)
            {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            exit_code: output.status.code(),
            elapsed_ms: started.elapsed().as_millis(),
            command: rendered_command(spec),
        },
        Err(_) => CheckRecord {
            name: spec.name,
            status: CheckStatus::CouldNotStart,
            exit_code: None,
            elapsed_ms: started.elapsed().as_millis(),
            command: rendered_command(spec),
        },
    }
}

fn rendered_command(spec: CheckSpec) -> Vec<String> {
    std::iter::once(spec.program.to_string())
        .chain(spec.toolchain.map(str::to_string))
        .chain(spec.args.iter().map(|argument| (*argument).to_string()))
        .collect()
}

fn bind_source(root: &Path) -> Result<SourceBinding, String> {
    let git_head = git_output(root, &["rev-parse", "HEAD"])?;
    let status = command_bytes(root, "git", &["status", "--porcelain=v1", "-z"])?;
    let paths = command_bytes(
        root,
        "git",
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let mut source = Sha256::new();
    source.update(b"seiri.completion.source.v2");
    digest_field(&mut source, &status);
    for raw in paths
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let relative = std::str::from_utf8(raw)
            .map_err(|_| "git returned a non-UTF-8 repository path".to_string())?;
        digest_field(&mut source, raw);
        match fs::read(root.join(relative)) {
            Ok(bytes) => {
                digest_field(&mut source, b"present");
                digest_field(&mut source, &bytes);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                digest_field(&mut source, b"deleted");
            }
            Err(error) => {
                return Err(format!("failed to bind source file '{relative}': {error}"));
            }
        }
    }
    let lock = fs::read(root.join("Cargo.lock"))
        .map_err(|error| format!("failed to bind Cargo.lock: {error}"))?;
    Ok(SourceBinding {
        git_head,
        worktree_dirty: !status.is_empty(),
        source_digest: format_digest(source.finalize().into()),
        cargo_lock_digest: format_digest(Sha256::digest(lock).into()),
    })
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let bytes = command_bytes(root, "git", args)?;
    String::from_utf8(bytes)
        .map(|value| value.trim().to_string())
        .map_err(|_| "git returned non-UTF-8 output".to_string())
}

fn command_bytes(root: &Path, program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to start {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{program} source-binding command failed"));
    }
    Ok(output.stdout)
}

fn digest_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn format_digest(bytes: [u8; 32]) -> String {
    let mut value = String::from("sha256:");
    for byte in bytes {
        use std::fmt::Write as _;
        write!(value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

fn validate_captured_output(name: &str, stdout: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(stdout) else {
        return !matches!(name, "self_audit_summary" | "self_audit_linter");
    };
    match name {
        "self_audit_summary" => {
            value["schema_version"] == seiri_core::CODEX_SCHEMA_VERSION
                && value["query"]["kind"] == "summary"
                && value["query"]["data"]["findings"] == 0
        }
        "self_audit_linter" => {
            value["schema_version"] == seiri_core::CODEX_SCHEMA_VERSION
                && value["query"]["kind"] == "linter"
                && value["query"]["data"]["report"]["summary"]["findings"] == 0
        }
        _ => true,
    }
}

fn option<'a>(args: &'a [OsString], name: &str) -> Result<&'a str, String> {
    optional_option(args, name).ok_or_else(|| format!("missing value for {name}"))
}

fn optional_option<'a>(args: &'a [OsString], name: &str) -> Option<&'a str> {
    let index = args.iter().position(|value| value == name)?;
    args.get(index + 1)?.to_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_registry_is_unique_and_nonempty() {
        let mut names = CHECKS.iter().map(|check| check.name).collect::<Vec<_>>();
        assert!(names.iter().all(|name| !name.is_empty()));
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
        assert!(CHECKS.len() >= 10);
        assert!(CHECKS.iter().all(|check| {
            check.program != "cargo" || check.name == "format" || check.args.contains(&"--locked")
        }));
    }

    #[test]
    fn self_audit_output_validation_fails_closed() {
        assert!(!validate_captured_output("self_audit_summary", b"{}"));
        assert!(!validate_captured_output("self_audit_linter", b"not-json"));
        assert!(validate_captured_output("workspace_tests", b"not-json"));
    }
}
