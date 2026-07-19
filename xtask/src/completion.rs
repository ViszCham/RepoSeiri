use crate::{bundle, calibration, repository_root, supervisor};
use seiri_report::{EmpiricalCalibrationStatus, HoldoutCalibrationReport};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

const CHECK_OUTPUT_LIMIT_BYTES: usize = 8 * 1024 * 1024;
const GIT_OUTPUT_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const REQUIRED_IMPLEMENTATION_PATHS: &[&str] = &[
    "crates/seiri-core/src/contracts.rs",
    "crates/seiri-core/src/document_index.rs",
    "crates/seiri-core/src/document_scan.rs",
    "crates/seiri-core/src/evidence_kernel.rs",
    "crates/seiri-core/src/pattern_extension.rs",
    "crates/seiri-fs/src/walker.rs",
    "crates/seiri-markdown/src/classifier.rs",
    "crates/seiri-markdown/src/source.rs",
    "crates/seiri-patterns/src/executable/evaluation.rs",
    "crates/seiri-report/src/holdout.rs",
    "crates/seiri-report/src/propositions.rs",
    "xtask/src/bundle.rs",
    "xtask/src/calibration.rs",
    "xtask/src/completion.rs",
    "xtask/src/supervisor.rs",
    "schemas/seiri.completion.v3.json",
    "schemas/seiri.calibration-corpus.v1.json",
    "schemas/seiri.calibration-holdout.v1.json",
    "fixtures/calibration-holdout-corpus.v1.json",
    "tests/calibration_holdout.rs",
    "tests/product_surface.rs",
    "tests/r10_sip_protocol.rs",
];

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
        "fuzz_targets_compile",
        &[
            "check",
            "--manifest-path",
            "fuzz/Cargo.toml",
            "--bins",
            "--locked",
        ],
    ),
    CheckSpec::cargo_toolchain(
        "fuzz_markdown_smoke",
        "+nightly-2026-07-01",
        &[
            "--locked",
            "fuzz",
            "run",
            "markdown",
            "--",
            "-runs=64",
            "-max_len=65536",
        ],
    ),
    CheckSpec::cargo_toolchain(
        "fuzz_bounded_reader_smoke",
        "+nightly-2026-07-01",
        &[
            "--locked",
            "fuzz",
            "run",
            "calibration_jsonl",
            "--",
            "-runs=64",
            "-max_len=65536",
        ],
    ),
    CheckSpec::cargo_toolchain(
        "fuzz_pack_compiler_smoke",
        "+nightly-2026-07-01",
        &[
            "--locked",
            "fuzz",
            "run",
            "executable_pack",
            "--",
            "-runs=64",
            "-max_len=65536",
        ],
    ),
    CheckSpec::cargo_toolchain(
        "fuzz_schema_decoder_smoke",
        "+nightly-2026-07-01",
        &[
            "--locked",
            "fuzz",
            "run",
            "schema_decoder",
            "--",
            "-runs=64",
            "-max_len=65536",
        ],
    ),
    CheckSpec::cargo_toolchain(
        "fuzz_delta_smoke",
        "+nightly-2026-07-01",
        &[
            "--locked",
            "fuzz",
            "run",
            "audit_delta",
            "--",
            "-runs=64",
            "-max_len=65536",
        ],
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
    timeout_secs: u64,
}

impl CheckSpec {
    const fn cargo(name: &'static str, args: &'static [&'static str]) -> Self {
        Self {
            name,
            program: "cargo",
            toolchain: None,
            args,
            timeout_secs: 15 * 60,
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
            timeout_secs: 15 * 60,
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
            timeout_secs: 60,
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
    calibration: Option<HoldoutCalibrationReport>,
    claims: Vec<CompletionClaimRecord>,
    evidence_complete: bool,
    skipped_checks: Vec<String>,
    boundary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CompletionState {
    ReadyForGit,
    ImplementedWithBlockedEvidence,
    Incomplete,
}

#[derive(Debug, Serialize)]
struct CheckRecord {
    name: &'static str,
    status: CheckStatus,
    failure_class: Option<supervisor::ProcessFailureKind>,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    command: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CompletionClaimRecord {
    kind: CompletionClaimKind,
    status: CompletionClaimStatus,
    evidence: Vec<String>,
    boundary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CompletionClaimKind {
    Implemented,
    LocallyVerified,
    HostVerified,
    Calibrated,
    ManualPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CompletionClaimStatus {
    Satisfied,
    Unsatisfied,
    RequiresHumanDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SourceBinding {
    pub(crate) git_head: String,
    pub(crate) worktree_dirty: bool,
    pub(crate) source_digest: String,
    pub(crate) cargo_lock_digest: String,
}

struct Unbound;
struct Bound;

struct CompletionRun<State> {
    root: std::path::PathBuf,
    source: Option<SourceBinding>,
    _state: PhantomData<State>,
}

impl CompletionRun<Unbound> {
    fn new(root: std::path::PathBuf) -> Self {
        Self {
            root,
            source: None,
            _state: PhantomData,
        }
    }

    fn bind(self) -> Result<CompletionRun<Bound>, String> {
        let source = bind_source(&self.root)?;
        Ok(CompletionRun {
            root: self.root,
            source: Some(source),
            _state: PhantomData,
        })
    }
}

impl CompletionRun<Bound> {
    fn source(&self) -> &SourceBinding {
        self.source.as_ref().expect("bound completion has source")
    }

    fn unchanged(&self) -> Result<bool, String> {
        Ok(bind_source(&self.root)? == *self.source())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Passed,
    Failed,
}

pub fn run(args: &[OsString]) -> Result<ExitCode, String> {
    if option(args, "--format")? != "json" {
        return Err("completion supports only '--format json'".to_string());
    }
    let run = CompletionRun::<Unbound>::new(repository_root()?).bind()?;
    let root = &run.root;
    let (implemented, implementation_check) = implementation_surface(root);
    let mut checks = vec![implementation_check];
    checks.extend(
        CHECKS
            .iter()
            .map(|spec| run_check(root, *spec))
            .collect::<Vec<_>>(),
    );
    let calibration_started = std::time::Instant::now();
    let calibration = calibration::evaluate(root).ok();
    checks.push(CheckRecord {
        name: "holdout_report",
        status: if calibration.is_some() {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        failure_class: None,
        exit_code: None,
        elapsed_ms: calibration_started.elapsed().as_millis(),
        command: vec![
            "xtask".to_string(),
            "calibration-holdout".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ],
    });
    let host_evidence = requested_option(args, "--host-evidence")?;
    let required_hosts =
        bundle::validate_required_hosts(host_evidence.map(Path::new), run.source());
    checks.push(CheckRecord {
        name: "source_unchanged",
        status: if run.unchanged()? {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        failure_class: None,
        exit_code: None,
        elapsed_ms: 0,
        command: vec!["source-binding-pre-post".to_string()],
    });
    let locally_verified = checks
        .iter()
        .all(|check| check.status == CheckStatus::Passed);
    let host_verified = required_hosts
        .iter()
        .all(|host| host.status == bundle::HostEvidenceStatus::Passed);
    let calibrated = calibration
        .as_ref()
        .is_some_and(|report| report.status == EmpiricalCalibrationStatus::Calibrated);
    let evidence_complete =
        derive_evidence_complete(implemented, locally_verified, host_verified, calibrated);
    let state = derive_state(implemented, &checks, host_evidence.map(|_| host_verified));
    let claims = completion_claims(
        implemented,
        locally_verified,
        host_verified,
        calibrated,
        REQUIRED_IMPLEMENTATION_PATHS.len(),
    );
    let record = CompletionRecord {
        schema_version: seiri_core::COMPLETION_SCHEMA_VERSION,
        state,
        tool_version: env!("CARGO_PKG_VERSION"),
        source: run.source().clone(),
        checks,
        required_hosts,
        calibration,
        claims,
        evidence_complete,
        skipped_checks: Vec::new(),
        boundary: "completion reports one verified worktree state; it does not authorize commit, push, merge, release, plugin installation, restart, or visibility changes",
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&record).map_err(|error| error.to_string())?
    );
    Ok(match state {
        CompletionState::ReadyForGit => ExitCode::SUCCESS,
        CompletionState::ImplementedWithBlockedEvidence | CompletionState::Incomplete => {
            ExitCode::FAILURE
        }
    })
}

fn implementation_surface(root: &Path) -> (bool, CheckRecord) {
    let present = REQUIRED_IMPLEMENTATION_PATHS
        .iter()
        .filter(|relative| root.join(relative).is_file())
        .count();
    let implemented = present == REQUIRED_IMPLEMENTATION_PATHS.len();
    (
        implemented,
        CheckRecord {
            name: "implementation_surface",
            status: if implemented {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            failure_class: None,
            exit_code: None,
            elapsed_ms: 0,
            command: vec![format!(
                "required-implementation-surface:{present}/{}",
                REQUIRED_IMPLEMENTATION_PATHS.len()
            )],
        },
    )
}

fn derive_state(
    implemented: bool,
    checks: &[CheckRecord],
    required_host_verified: Option<bool>,
) -> CompletionState {
    if !implemented {
        return CompletionState::Incomplete;
    }
    if checks
        .iter()
        .all(|check| check.status == CheckStatus::Passed)
    {
        return if required_host_verified == Some(false) {
            CompletionState::ImplementedWithBlockedEvidence
        } else {
            CompletionState::ReadyForGit
        };
    }
    if checks
        .iter()
        .filter(|check| check.status == CheckStatus::Failed)
        .all(|check| check.failure_class.is_some_and(is_environment_blocked_kind))
    {
        CompletionState::ImplementedWithBlockedEvidence
    } else {
        CompletionState::Incomplete
    }
}

const fn is_environment_blocked_kind(kind: supervisor::ProcessFailureKind) -> bool {
    matches!(
        kind,
        supervisor::ProcessFailureKind::MissingExecutable
            | supervisor::ProcessFailureKind::CouldNotStart
            | supervisor::ProcessFailureKind::EnvironmentBlocked
    )
}

const fn derive_evidence_complete(
    implemented: bool,
    locally_verified: bool,
    host_verified: bool,
    calibrated: bool,
) -> bool {
    implemented && locally_verified && host_verified && calibrated
}

fn completion_claims(
    implemented: bool,
    locally_verified: bool,
    host_verified: bool,
    calibrated: bool,
    required_paths: usize,
) -> Vec<CompletionClaimRecord> {
    vec![
        CompletionClaimRecord {
            kind: CompletionClaimKind::Implemented,
            status: binary_claim_status(implemented),
            evidence: vec![format!(
                "C0-C8 required implementation surfaces: {required_paths}"
            )],
            boundary: "File presence is implementation-surface evidence; verification is reported separately.",
        },
        CompletionClaimRecord {
            kind: CompletionClaimKind::LocallyVerified,
            status: binary_claim_status(locally_verified),
            evidence: vec!["required local checks bound to this source".to_string()],
            boundary: "A blocked or failed local check is never promoted to pass.",
        },
        CompletionClaimRecord {
            kind: CompletionClaimKind::HostVerified,
            status: binary_claim_status(host_verified),
            evidence: vec![
                "source-bound Windows and Linux bundle receipts are both required".to_string(),
            ],
            boundary: "Host receipts verify the bounded command set, not general platform correctness.",
        },
        CompletionClaimRecord {
            kind: CompletionClaimKind::Calibrated,
            status: binary_claim_status(calibrated),
            evidence: vec![
                "each task requires an independent holdout sample at or above the declared minimum"
                    .to_string(),
            ],
            boundary: "Low-N synthetic fixture results remain insufficient_sample and are not general performance evidence.",
        },
        CompletionClaimRecord {
            kind: CompletionClaimKind::ManualPolicy,
            status: CompletionClaimStatus::RequiresHumanDecision,
            evidence: vec![
                "commit, push, merge, release, publication, visibility, installation, and restart"
                    .to_string(),
            ],
            boundary: "Completion records do not grant operational authority.",
        },
    ]
}

const fn binary_claim_status(satisfied: bool) -> CompletionClaimStatus {
    if satisfied {
        CompletionClaimStatus::Satisfied
    } else {
        CompletionClaimStatus::Unsatisfied
    }
}

fn run_check(root: &Path, spec: CheckSpec) -> CheckRecord {
    let mut args = Vec::<OsString>::new();
    if let Some(toolchain) = spec.toolchain {
        args.push(toolchain.into());
    }
    args.extend(spec.args.iter().map(OsString::from));
    let process = supervisor::ProcessSpec::new(spec.program)
        .args(&args)
        .current_dir(root)
        .timeout(Duration::from_secs(spec.timeout_secs))
        .output_limits(CHECK_OUTPUT_LIMIT_BYTES, CHECK_OUTPUT_LIMIT_BYTES);
    match supervisor::run(&process) {
        Ok(output) => CheckRecord {
            name: spec.name,
            status: if validate_captured_output(spec.name, &output.stdout) {
                CheckStatus::Passed
            } else {
                CheckStatus::Failed
            },
            failure_class: None,
            exit_code: output.status.code(),
            elapsed_ms: output.elapsed.as_millis(),
            command: process.rendered_command(),
        },
        Err(failure) => CheckRecord {
            name: spec.name,
            status: CheckStatus::Failed,
            failure_class: Some(classify_check_failure(spec, &failure)),
            exit_code: failure.exit_code,
            elapsed_ms: failure.elapsed.as_millis(),
            command: process.rendered_command(),
        },
    }
}

fn classify_check_failure(
    spec: CheckSpec,
    failure: &supervisor::ProcessFailure,
) -> supervisor::ProcessFailureKind {
    if failure.kind != supervisor::ProcessFailureKind::NonZeroExit {
        return failure.kind;
    }
    let mut message = String::from_utf8_lossy(&failure.stdout).to_lowercase();
    message.push_str(&String::from_utf8_lossy(&failure.stderr).to_lowercase());
    let application_control = [
        "os error 4551",
        "application control",
        "app control",
        "blocked by group policy",
        "アプリケーション制御",
    ]
    .iter()
    .any(|marker| message.contains(marker));
    let missing_toolchain = spec.toolchain.is_some()
        && message.contains("toolchain")
        && (message.contains("is not installed")
            || message.contains("not installed")
            || message.contains("not found"));
    let missing_cargo_fuzz = spec.name.starts_with("fuzz_")
        && (message.contains("no such command: `fuzz`")
            || message.contains("no such command: fuzz")
            || (message.contains("cargo-fuzz")
                && (message.contains("not installed") || message.contains("not found"))));
    if application_control || missing_toolchain || missing_cargo_fuzz {
        supervisor::ProcessFailureKind::EnvironmentBlocked
    } else {
        failure.kind
    }
}

pub(crate) fn bind_source(root: &Path) -> Result<SourceBinding, String> {
    let git_head = git_output(root, &["rev-parse", "HEAD"])?;
    let status = command_bytes(root, "git", &["status", "--porcelain=v1", "-z"])?;
    let tracked = command_bytes(root, "git", &["ls-files", "--stage", "-z"])?;
    let untracked = command_bytes(
        root,
        "git",
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?;
    let mut source = Sha256::new();
    source.update(b"seiri.completion.source.v3");
    digest_field(&mut source, &status);
    let mut total_bytes = 0u64;
    for record in nul_records(&tracked) {
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| "git returned a malformed index record".to_string())?;
        let (metadata, path_with_tab) = record.split_at(separator);
        let raw_path = &path_with_tab[1..];
        let relative = std::str::from_utf8(raw_path)
            .map_err(|_| "git returned a non-UTF-8 repository path".to_string())?;
        digest_field(&mut source, metadata);
        digest_field(&mut source, raw_path);
        bind_worktree_entry(
            root,
            relative,
            metadata.starts_with(b"160000 "),
            &mut source,
            &mut total_bytes,
        )?;
    }
    for raw_path in nul_records(&untracked) {
        let relative = std::str::from_utf8(raw_path)
            .map_err(|_| "git returned a non-UTF-8 repository path".to_string())?;
        digest_field(&mut source, b"untracked");
        digest_field(&mut source, raw_path);
        bind_worktree_entry(root, relative, false, &mut source, &mut total_bytes)?;
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

fn nul_records(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|item| !item.is_empty())
}

fn bind_worktree_entry(
    root: &Path,
    relative: &str,
    gitlink: bool,
    digest: &mut Sha256,
    total_bytes: &mut u64,
) -> Result<(), String> {
    const MAX_SOURCE_FILE_BYTES: u64 = 64 * 1024 * 1024;
    const MAX_TOTAL_SOURCE_BYTES: u64 = 1024 * 1024 * 1024;
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            digest_field(digest, b"deleted");
            return Ok(());
        }
        Err(error) => return Err(format!("failed to inspect repository source: {error}")),
    };
    if gitlink {
        digest_field(digest, b"gitlink");
        return Ok(());
    }
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(&path)
            .map_err(|error| format!("failed to read repository symlink: {error}"))?;
        let target = target
            .to_str()
            .ok_or_else(|| "repository symlink target is not UTF-8".to_string())?;
        digest_field(digest, b"symlink");
        digest_field(digest, target.as_bytes());
        return Ok(());
    }
    if !metadata.is_file() {
        return Err("tracked source is neither a regular file, symlink, nor gitlink".to_string());
    }
    if metadata.len() > MAX_SOURCE_FILE_BYTES {
        return Err("repository source file exceeds completion byte limit".to_string());
    }
    *total_bytes = total_bytes.saturating_add(metadata.len());
    if *total_bytes > MAX_TOTAL_SOURCE_BYTES {
        return Err("repository source exceeds completion total byte limit".to_string());
    }
    digest_field(digest, b"regular");
    let mut file =
        File::open(&path).map_err(|error| format!("failed to open repository source: {error}"))?;
    let mut file_digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read repository source: {error}"))?;
        if read == 0 {
            break;
        }
        file_digest.update(&buffer[..read]);
    }
    digest_field(digest, &file_digest.finalize());
    Ok(())
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let bytes = command_bytes(root, "git", args)?;
    String::from_utf8(bytes)
        .map(|value| value.trim().to_string())
        .map_err(|_| "git returned non-UTF-8 output".to_string())
}

fn command_bytes(root: &Path, program: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let process = supervisor::ProcessSpec::new(program)
        .args(args)
        .current_dir(root)
        .timeout(Duration::from_secs(30))
        .output_limits(GIT_OUTPUT_LIMIT_BYTES, CHECK_OUTPUT_LIMIT_BYTES);
    supervisor::run(&process)
        .map(|output| output.stdout)
        .map_err(|failure| {
            format!(
                "{program} source-binding command failed: {:?}",
                failure.kind
            )
        })
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
                && value["query"]["data"]["documents"]["primary_skipped_document_budget"] == 0
                && value["query"]["data"]["documents"]["primary_skipped_byte_budget"] == 0
                && value["query"]["data"]["coverage"]["partial_scopes"] == 0
                && value["query"]["data"]["coverage"]["markdown_documents"]["kind"] == "complete"
                && value["query"]["data"]["coverage"]["conflict_coverage"]["kind"] == "complete"
                && value["query"]["data"]["observations"]["unacknowledged_unknown"] == 0
                && value["query"]["data"]["observations"]["conflict"] == 0
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

fn requested_option<'a>(args: &'a [OsString], name: &str) -> Result<Option<&'a str>, String> {
    if args.iter().any(|value| value == name) {
        option(args, name).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_check(
        status: CheckStatus,
        failure_class: Option<supervisor::ProcessFailureKind>,
    ) -> CheckRecord {
        CheckRecord {
            name: "synthetic",
            status,
            failure_class,
            exit_code: None,
            elapsed_ms: 0,
            command: vec!["synthetic".to_string()],
        }
    }

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
        let incomplete = serde_json::json!({
            "schema_version": seiri_core::CODEX_SCHEMA_VERSION,
            "query": {
                "kind": "summary",
                "data": {
                    "findings": 0,
                    "documents": {
                        "primary_skipped_document_budget": 0,
                        "primary_skipped_byte_budget": 0
                    },
                    "coverage": {
                        "partial_scopes": 0,
                        "markdown_documents": { "kind": "complete" },
                        "conflict_coverage": { "kind": "complete" }
                    },
                    "observations": {
                        "unknown": 1,
                        "unacknowledged_unknown": 1,
                        "conflict": 0
                    }
                }
            }
        });
        assert!(!validate_captured_output(
            "self_audit_summary",
            serde_json::to_string(&incomplete)
                .expect("summary fixture")
                .as_bytes()
        ));
    }

    #[test]
    fn source_binding_is_repeatable_and_canonical() {
        let root = repository_root().expect("repository root");
        let first = bind_source(&root).expect("first binding");
        let second = bind_source(&root).expect("second binding");
        assert_eq!(first, second);
        assert!(first.source_digest.starts_with("sha256:"));
        assert_eq!(first.source_digest.len(), 71);
        assert!(first.cargo_lock_digest.starts_with("sha256:"));
    }

    #[test]
    fn completion_state_separates_local_pass_blocked_environment_and_failure() {
        assert_eq!(
            derive_state(true, &[synthetic_check(CheckStatus::Passed, None)], None),
            CompletionState::ReadyForGit
        );
        assert_eq!(
            derive_state(
                true,
                &[synthetic_check(CheckStatus::Passed, None)],
                Some(true)
            ),
            CompletionState::ReadyForGit
        );
        assert_eq!(
            derive_state(
                true,
                &[synthetic_check(CheckStatus::Passed, None)],
                Some(false)
            ),
            CompletionState::ImplementedWithBlockedEvidence
        );

        let blocked = synthetic_check(
            CheckStatus::Failed,
            Some(supervisor::ProcessFailureKind::EnvironmentBlocked),
        );
        assert_eq!(
            derive_state(true, &[blocked], None),
            CompletionState::ImplementedWithBlockedEvidence
        );

        let failed = synthetic_check(
            CheckStatus::Failed,
            Some(supervisor::ProcessFailureKind::NonZeroExit),
        );
        assert_eq!(
            derive_state(true, &[failed], Some(false)),
            CompletionState::Incomplete
        );
        assert_eq!(
            derive_state(
                false,
                &[synthetic_check(CheckStatus::Passed, None)],
                Some(true)
            ),
            CompletionState::Incomplete
        );
    }

    #[test]
    fn explicitly_requested_host_evidence_requires_a_value() {
        let missing = [OsString::from("--host-evidence")];
        assert_eq!(
            requested_option(&missing, "--host-evidence"),
            Err("missing value for --host-evidence".to_string())
        );
        let present = [
            OsString::from("--host-evidence"),
            OsString::from("target/host-evidence"),
        ];
        assert_eq!(
            requested_option(&present, "--host-evidence"),
            Ok(Some("target/host-evidence"))
        );
        assert_eq!(requested_option(&[], "--host-evidence"), Ok(None));
    }

    #[test]
    fn evidence_complete_requires_local_host_and_calibration_evidence() {
        assert!(!derive_evidence_complete(true, true, false, true));
        assert!(!derive_evidence_complete(true, true, true, false));
        assert!(derive_evidence_complete(true, true, true, true));
        let claims = completion_claims(true, true, false, false, 22);
        assert_eq!(claims.len(), 5);
        assert_eq!(
            claims
                .iter()
                .find(|claim| claim.kind == CompletionClaimKind::ManualPolicy)
                .expect("manual policy claim")
                .status,
            CompletionClaimStatus::RequiresHumanDecision
        );
    }

    #[test]
    fn environment_markers_are_narrowly_reclassified() {
        let spec = CheckSpec::cargo_toolchain(
            "fuzz_schema_decoder_smoke",
            "+nightly-2026-07-01",
            &["--locked", "fuzz"],
        );
        let missing = supervisor::ProcessFailure {
            kind: supervisor::ProcessFailureKind::NonZeroExit,
            exit_code: Some(1),
            stdout: Vec::new(),
            stderr: b"toolchain 'nightly-2026-07-01' is not installed".to_vec(),
            elapsed: Duration::ZERO,
        };
        assert_eq!(
            classify_check_failure(spec, &missing),
            supervisor::ProcessFailureKind::EnvironmentBlocked
        );

        let crash = supervisor::ProcessFailure {
            kind: supervisor::ProcessFailureKind::NonZeroExit,
            exit_code: Some(1),
            stdout: Vec::new(),
            stderr: b"fuzz target found a reproducible assertion failure".to_vec(),
            elapsed: Duration::ZERO,
        };
        assert_eq!(
            classify_check_failure(spec, &crash),
            supervisor::ProcessFailureKind::NonZeroExit
        );
    }
}
