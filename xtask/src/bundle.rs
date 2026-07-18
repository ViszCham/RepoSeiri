use crate::{repository_root, supervisor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

const RUNTIME_MANIFEST_SCHEMA: &str = "reposeiri.runtime-manifest.v3";
const BUNDLE_METADATA_VERSION: &str = "reposeiri.bundle-metadata.v1";
const HOST_COMMAND_SET: [&str; 4] = [
    "native_contract",
    "native_codex_summary",
    "schema_integrity",
    "launcher_codex_summary",
];
const PROCESS_OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeManifest {
    schema_version: String,
    bundle_metadata_version: String,
    tool_version: String,
    target: String,
    binary: String,
    sha256: String,
    contract_schema: String,
    analysis_schema: String,
    patch_plan_schema: String,
    codex_schema: String,
    error_schema: String,
    completion_schema: String,
    portable_audit_schema: String,
    audit_delta_schema: String,
    semantic_revisions: seiri_core::SemanticRevisions,
    standalone_smoke: String,
    source_digest: String,
    cargo_lock_digest: String,
    command_set: Vec<String>,
    schema_sha256: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct HostEvidenceRecord {
    pub target: &'static str,
    pub status: HostEvidenceStatus,
    pub source_digest: Option<String>,
    pub cargo_lock_digest: Option<String>,
    pub binary_digest: Option<String>,
    pub command_set: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostEvidenceStatus {
    Passed,
    Missing,
    Invalid,
}

pub fn run(args: &[OsString]) -> Result<ExitCode, String> {
    let target = option(args, "--target")?;
    let binary = PathBuf::from(option(args, "--binary")?);
    let output = PathBuf::from(option(args, "--output")?);
    if !binary.is_file() {
        return Err("--binary must name an existing regular file".to_string());
    }
    if output.exists() {
        return Err("--output must not already exist".to_string());
    }
    let root = repository_root()?;
    let source = crate::completion::bind_source(&root)?;
    validate_plugin_surface(&root.join("plugins/reposeiri"))?;
    let plugin_output = output.join("reposeiri");
    copy_tree(&root.join("plugins/reposeiri"), &plugin_output)?;
    copy_tree(&root.join("schemas"), &plugin_output.join("schemas"))?;
    let binary_name = if target.contains("windows") {
        "seiri.exe"
    } else {
        "seiri"
    };
    let bundled_binary = plugin_output.join("bin").join(binary_name);
    fs::create_dir_all(
        bundled_binary
            .parent()
            .ok_or_else(|| "bundle binary has no parent".to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::copy(&binary, &bundled_binary).map_err(|error| error.to_string())?;
    smoke_native(&bundled_binary, target)?;
    let schema_sha256 = schema_digests(&plugin_output.join("schemas"))?;
    let manifest = RuntimeManifest {
        schema_version: RUNTIME_MANIFEST_SCHEMA.to_string(),
        bundle_metadata_version: BUNDLE_METADATA_VERSION.to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        target: target.to_string(),
        binary: format!("bin/{binary_name}"),
        sha256: sha256_file(&bundled_binary)?,
        contract_schema: seiri_core::CONTRACT_SCHEMA_VERSION.to_string(),
        analysis_schema: seiri_core::ANALYSIS_SCHEMA_VERSION.to_string(),
        patch_plan_schema: seiri_core::PATCH_PLAN_SCHEMA_VERSION.to_string(),
        codex_schema: seiri_core::CODEX_SCHEMA_VERSION.to_string(),
        error_schema: seiri_core::ERROR_SCHEMA_VERSION.to_string(),
        completion_schema: seiri_core::COMPLETION_SCHEMA_VERSION.to_string(),
        portable_audit_schema: seiri_core::PORTABLE_AUDIT_SCHEMA_VERSION.to_string(),
        audit_delta_schema: seiri_core::AUDIT_DELTA_SCHEMA_VERSION.to_string(),
        semantic_revisions: seiri_core::SemanticRevisions::default(),
        standalone_smoke: "passed".to_string(),
        source_digest: source.source_digest.clone(),
        cargo_lock_digest: source.cargo_lock_digest.clone(),
        command_set: HOST_COMMAND_SET.iter().map(ToString::to_string).collect(),
        schema_sha256,
    };
    fs::write(
        plugin_output.join("runtime-manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    validate_bundle_surface(&plugin_output, &source)?;
    smoke_launcher(&plugin_output, &bundled_binary, target)?;
    validate_bundle_surface(&plugin_output, &source)?;
    println!(
        "{}",
        serde_json::to_string(&manifest).map_err(|error| error.to_string())?
    );
    Ok(ExitCode::SUCCESS)
}

pub fn validate_required_hosts(
    directory: Option<&Path>,
    expected: &crate::completion::SourceBinding,
) -> Vec<HostEvidenceRecord> {
    const REQUIRED: [&str; 2] = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];
    let manifests = directory.map(find_runtime_manifests).unwrap_or_default();
    REQUIRED
        .into_iter()
        .map(|target| {
            let matching = manifests.iter().find_map(|path| {
                let manifest =
                    serde_json::from_slice::<RuntimeManifest>(&fs::read(path).ok()?).ok()?;
                (manifest.target == target).then_some((path, manifest))
            });
            let (status, binding) = match matching {
                None => (HostEvidenceStatus::Missing, None),
                Some((path, manifest)) => {
                    let root = path.parent().unwrap_or_else(|| Path::new("."));
                    if validate_runtime_manifest(root, &manifest, expected).is_ok() {
                        let binding = HostReceiptBinding {
                            source_digest: manifest.source_digest,
                            cargo_lock_digest: manifest.cargo_lock_digest,
                            binary_digest: manifest.sha256,
                            command_set: manifest.command_set,
                        };
                        (HostEvidenceStatus::Passed, Some(binding))
                    } else {
                        (HostEvidenceStatus::Invalid, None)
                    }
                }
            };
            HostEvidenceRecord {
                target,
                status,
                source_digest: binding
                    .as_ref()
                    .map(|receipt| receipt.source_digest.clone()),
                cargo_lock_digest: binding
                    .as_ref()
                    .map(|receipt| receipt.cargo_lock_digest.clone()),
                binary_digest: binding
                    .as_ref()
                    .map(|receipt| receipt.binary_digest.clone()),
                command_set: binding
                    .map(|receipt| receipt.command_set)
                    .unwrap_or_default(),
            }
        })
        .collect()
}

struct HostReceiptBinding {
    source_digest: String,
    cargo_lock_digest: String,
    binary_digest: String,
    command_set: Vec<String>,
}

fn smoke_native(binary: &Path, target: &str) -> Result<(), String> {
    let current = std::env::current_dir().map_err(|error| error.to_string())?;
    let binary = absolute_from(&current, binary);
    let smoke_root = create_smoke_root(target)?;
    let contract = supervisor::run(
        &supervisor::ProcessSpec::new(&binary)
            .args(["contract", "--format", "json"])
            .current_dir(&smoke_root)
            .timeout(Duration::from_secs(30))
            .output_limits(PROCESS_OUTPUT_LIMIT, PROCESS_OUTPUT_LIMIT),
    )
    .map_err(process_failure)?;
    reject_unexpected_stderr(&contract.stderr)?;
    let contract: seiri_core::ContractManifest = serde_json::from_slice(&contract.stdout)
        .map_err(|_| "native contract smoke returned invalid JSON".to_string())?;
    contract
        .validate_current()
        .map_err(|_| "native contract smoke returned an unsupported contract".to_string())?;
    if contract.tool_version != env!("CARGO_PKG_VERSION") {
        return Err("native contract smoke returned a different tool version".to_string());
    }
    let summary = supervisor::run(
        &supervisor::ProcessSpec::new(&binary)
            .args([
                "codex", "--path", ".", "--scope", "subtree", "--query", "summary", "--format",
                "json",
            ])
            .current_dir(&smoke_root)
            .timeout(Duration::from_secs(30))
            .output_limits(PROCESS_OUTPUT_LIMIT, PROCESS_OUTPUT_LIMIT),
    )
    .map_err(process_failure)?;
    reject_unexpected_stderr(&summary.stderr)?;
    validate_summary(&summary.stdout)?;
    fs::remove_dir_all(&smoke_root).map_err(|error| error.to_string())
}

fn smoke_launcher(plugin_root: &Path, binary: &Path, target: &str) -> Result<(), String> {
    let current = std::env::current_dir().map_err(|error| error.to_string())?;
    let plugin_root = absolute_from(&current, plugin_root);
    let binary = absolute_from(&current, binary);
    let smoke_root = create_smoke_root(target)?;
    let process = if target.contains("windows") {
        supervisor::ProcessSpec::new("powershell").args([
            "-NoProfile",
            "-File",
            plugin_root
                .join("scripts/reposeiri-codex.ps1")
                .to_str()
                .ok_or_else(|| "bundle launcher path is not UTF-8".to_string())?,
            "-Path",
            ".",
            "-Query",
            "summary",
            "-Format",
            "json",
            "-Scope",
            "subtree",
        ])
    } else {
        supervisor::ProcessSpec::new("sh").args([
            plugin_root
                .join("scripts/reposeiri-codex.sh")
                .into_os_string(),
            "--path".into(),
            ".".into(),
            "--scope".into(),
            "subtree".into(),
            "--query".into(),
            "summary".into(),
            "--format".into(),
            "json".into(),
        ])
    };
    let output = supervisor::run(
        &process
            .current_dir(&smoke_root)
            .env("REPOSEIRI_BIN", binary)
            .timeout(Duration::from_secs(30))
            .output_limits(PROCESS_OUTPUT_LIMIT, PROCESS_OUTPUT_LIMIT),
    )
    .map_err(process_failure)?;
    reject_unexpected_stderr(&output.stderr)?;
    fs::remove_dir_all(&smoke_root).map_err(|error| error.to_string())?;
    validate_summary(&output.stdout)
}

fn create_smoke_root(target: &str) -> Result<PathBuf, String> {
    let smoke_root = std::env::temp_dir().join(format!(
        "reposeiri-bundle-smoke-{}-{}",
        std::process::id(),
        target.replace(['/', '\\'], "-")
    ));
    if smoke_root.exists() {
        return Err("bundle smoke directory already exists".to_string());
    }
    fs::create_dir_all(&smoke_root).map_err(|error| error.to_string())?;
    fs::write(smoke_root.join("README.md"), "# Bundle smoke\n")
        .map_err(|error| error.to_string())?;
    Ok(smoke_root)
}

fn validate_summary(stdout: &[u8]) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|_| "bundle summary smoke returned invalid JSON".to_string())?;
    if value["schema_version"] != seiri_core::CODEX_SCHEMA_VERSION
        || value["query"]["kind"] != "summary"
    {
        return Err("bundle summary smoke returned an unsupported contract".to_string());
    }
    Ok(())
}

fn reject_unexpected_stderr(stderr: &[u8]) -> Result<(), String> {
    if stderr.is_empty() {
        Ok(())
    } else {
        Err("bundle smoke emitted unexpected stderr".to_string())
    }
}

fn process_failure(failure: supervisor::ProcessFailure) -> String {
    let error_code =
        structured_error_code(&failure.stderr).unwrap_or_else(|| "unclassified".to_string());
    let shell_error_id =
        powershell_error_id(&failure.stderr).unwrap_or_else(|| "unclassified".to_string());
    let shell_command =
        powershell_command_class(&failure.stderr).unwrap_or_else(|| "unclassified".to_string());
    format!(
        "bundle smoke process failed: {:?}; error_code={error_code}; shell_error_id={shell_error_id}; shell_command={shell_command}; captured stdout={} stderr={} bytes",
        failure.kind,
        failure.stdout.len(),
        failure.stderr.len()
    )
}

fn structured_error_code(stderr: &[u8]) -> Option<String> {
    const MARKER: &str = "\"code\":\"";
    let stderr = String::from_utf8_lossy(stderr);
    bounded_diagnostic_token(stderr.split_once(MARKER)?.1.split_once('"')?.0, 64)
}

fn powershell_error_id(stderr: &[u8]) -> Option<String> {
    const MARKER: &str = "FullyQualifiedErrorId";
    let stderr = String::from_utf8_lossy(stderr);
    let line = stderr
        .lines()
        .find(|line| line.contains(MARKER))?
        .split_once(':')?
        .1
        .trim();
    bounded_diagnostic_token(line, 160)
}

fn powershell_command_class(stderr: &[u8]) -> Option<String> {
    let stderr = String::from_utf8_lossy(stderr);
    let command = stderr.lines().next()?.split_once(" : ")?.0.trim();
    if command
        .bytes()
        .any(|byte| matches!(byte, b'\\' | b'/' | b':'))
    {
        return Some("path_command".to_string());
    }
    bounded_diagnostic_token(command, 64)
}

fn bounded_diagnostic_token(value: &str, max_len: usize) -> Option<String> {
    if value.is_empty()
        || value.len() > max_len
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b','))
    {
        return None;
    }
    Some(value.to_string())
}

fn validate_plugin_surface(plugin_root: &Path) -> Result<(), String> {
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|_| "plugin manifest is missing".to_string())?,
    )
    .map_err(|_| "plugin manifest is invalid".to_string())?;
    if manifest["name"] != "reposeiri"
        || manifest["version"] != env!("CARGO_PKG_VERSION")
        || manifest["skills"] != "./skills/"
    {
        return Err("plugin manifest does not match the source contract".to_string());
    }
    for relative in [
        "skills/reposeiri/SKILL.md",
        "scripts/reposeiri-codex.ps1",
        "scripts/reposeiri-codex.sh",
    ] {
        let path = plugin_root.join(relative);
        let metadata =
            fs::symlink_metadata(path).map_err(|_| "plugin surface is incomplete".to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("plugin surface contains an invalid required entry".to_string());
        }
    }
    Ok(())
}

fn validate_bundle_surface(
    plugin_root: &Path,
    expected: &crate::completion::SourceBinding,
) -> Result<(), String> {
    validate_plugin_surface(plugin_root)?;
    let manifest: RuntimeManifest = serde_json::from_slice(
        &fs::read(plugin_root.join("runtime-manifest.json"))
            .map_err(|_| "runtime manifest is missing".to_string())?,
    )
    .map_err(|_| "runtime manifest is invalid".to_string())?;
    validate_runtime_manifest(plugin_root, &manifest, expected)
}

fn validate_runtime_manifest(
    plugin_root: &Path,
    manifest: &RuntimeManifest,
    expected: &crate::completion::SourceBinding,
) -> Result<(), String> {
    let binary_path = Path::new(&manifest.binary);
    if binary_path.is_absolute()
        || binary_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err("runtime manifest binary path is not portable".to_string());
    }
    let expected_commands = HOST_COMMAND_SET
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let schemas = schema_digests(&plugin_root.join("schemas"))?;
    if manifest.schema_version != RUNTIME_MANIFEST_SCHEMA
        || manifest.bundle_metadata_version != BUNDLE_METADATA_VERSION
        || manifest.tool_version != env!("CARGO_PKG_VERSION")
        || manifest.contract_schema != seiri_core::CONTRACT_SCHEMA_VERSION
        || manifest.analysis_schema != seiri_core::ANALYSIS_SCHEMA_VERSION
        || manifest.patch_plan_schema != seiri_core::PATCH_PLAN_SCHEMA_VERSION
        || manifest.codex_schema != seiri_core::CODEX_SCHEMA_VERSION
        || manifest.error_schema != seiri_core::ERROR_SCHEMA_VERSION
        || manifest.completion_schema != seiri_core::COMPLETION_SCHEMA_VERSION
        || manifest.portable_audit_schema != seiri_core::PORTABLE_AUDIT_SCHEMA_VERSION
        || manifest.audit_delta_schema != seiri_core::AUDIT_DELTA_SCHEMA_VERSION
        || manifest.semantic_revisions.validate_current().is_err()
        || manifest.standalone_smoke != "passed"
        || manifest.source_digest != expected.source_digest
        || manifest.cargo_lock_digest != expected.cargo_lock_digest
        || manifest.command_set != expected_commands
        || manifest.schema_sha256 != schemas
        || sha256_file(&plugin_root.join(binary_path)).ok().as_deref()
            != Some(manifest.sha256.as_str())
    {
        return Err("runtime manifest does not match the bundle contract".to_string());
    }
    Ok(())
}

fn schema_digests(schema_root: &Path) -> Result<BTreeMap<String, String>, String> {
    const MAX_SCHEMAS: usize = 128;
    let mut output = BTreeMap::new();
    for entry in fs::read_dir(schema_root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err("bundle schema directory contains a non-file entry".to_string());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "bundle schema name is not UTF-8".to_string())?;
        if !name.ends_with(".json") || output.len() >= MAX_SCHEMAS {
            return Err("bundle schema set is invalid or exceeds its limit".to_string());
        }
        output.insert(name, sha256_file(&entry.path())?);
    }
    for required in [
        "seiri.analysis.v2.json",
        "seiri.patch-plan.v2.json",
        "seiri.codex.v2.json",
        "seiri.error.v1.json",
        "seiri.completion.v3.json",
        "seiri.portable-audit.v2.json",
        "seiri.audit-delta.v2.json",
        "seiri.calibration-corpus.v1.json",
        "seiri.calibration-holdout.v1.json",
    ] {
        if !output.contains_key(required) {
            return Err("bundle schema set is incomplete".to_string());
        }
    }
    Ok(output)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let mut entries_seen = 0usize;
    copy_tree_inner(source, destination, 0, &mut entries_seen)
}

fn copy_tree_inner(
    source: &Path,
    destination: &Path,
    depth: usize,
    entries_seen: &mut usize,
) -> Result<(), String> {
    const MAX_COPY_DEPTH: usize = 16;
    const MAX_COPY_ENTRIES: usize = 4096;
    if depth > MAX_COPY_DEPTH {
        return Err("plugin bundle copy depth exceeded".to_string());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        *entries_seen += 1;
        if *entries_seen > MAX_COPY_ENTRIES {
            return Err("plugin bundle copy entry limit exceeded".to_string());
        }
        entries.push(entry.map_err(|error| error.to_string())?);
    }
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            return Err("plugin bundles do not follow symbolic links".to_string());
        }
        if file_type.is_dir() {
            copy_tree_inner(&entry.path(), &target, depth + 1, entries_seen)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn find_runtime_manifests(root: &Path) -> Vec<PathBuf> {
    fn visit(path: &Path, depth: usize, output: &mut Vec<PathBuf>) {
        if depth > 4 {
            return;
        }
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, depth + 1, output);
            } else if path.file_name().and_then(|name| name.to_str())
                == Some("runtime-manifest.json")
            {
                output.push(path);
            }
        }
    }
    let mut manifests = Vec::new();
    visit(root, 0, &mut manifests);
    manifests.sort();
    manifests
}

fn option<'a>(args: &'a [OsString], name: &str) -> Result<&'a str, String> {
    let index = args
        .iter()
        .position(|value| value == name)
        .ok_or_else(|| format!("missing {name}"))?;
    args.get(index + 1)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("missing value for {name}"))
}

fn absolute_from(current: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::SourceBinding;

    #[test]
    fn missing_host_evidence_fails_closed() {
        let source = SourceBinding {
            git_head: "test".to_string(),
            worktree_dirty: false,
            source_digest: "sha256:test".to_string(),
            cargo_lock_digest: "sha256:test".to_string(),
        };
        let hosts = validate_required_hosts(None, &source);
        assert_eq!(hosts.len(), 2);
        assert!(hosts
            .iter()
            .all(|host| host.status == HostEvidenceStatus::Missing));
    }

    #[test]
    fn bundle_options_require_explicit_values() {
        let args = vec![OsString::from("--target"), OsString::from("x")];
        assert_eq!(option(&args, "--target").expect("target"), "x");
        assert!(option(&args, "--binary").is_err());
    }

    #[test]
    fn launcher_failure_diagnostics_expose_only_bounded_error_codes() {
        let stderr = br#"Write-RepoSeiriError : {"schema_version":"seiri.error.v1","class":"contract","code":"schema_mismatch","message":"host path omitted"}"#;
        assert_eq!(
            structured_error_code(stderr).as_deref(),
            Some("schema_mismatch")
        );
        assert!(structured_error_code(
            br#"{"code":"C:\Users\name\private","message":"not portable"}"#
        )
        .is_none());
        assert!(structured_error_code(br#"C:\Users\name\private"#).is_none());
        assert_eq!(
            powershell_error_id(
                b"    + FullyQualifiedErrorId : InvalidOperation,Microsoft.PowerShell.Commands.WriteErrorException\r\n"
            )
            .as_deref(),
            Some("InvalidOperation,Microsoft.PowerShell.Commands.WriteErrorException")
        );
        assert!(
            powershell_error_id(b"FullyQualifiedErrorId : C:\\Users\\name\\private\r\n").is_none()
        );
        assert_eq!(
            powershell_command_class(
                b"Get-FileHash : The term 'Get-FileHash' is not recognized\r\n"
            )
            .as_deref(),
            Some("Get-FileHash")
        );
        assert_eq!(
            powershell_command_class(
                b"C:\\Users\\name\\private\\seiri.exe : The term was not recognized\r\n"
            )
            .as_deref(),
            Some("path_command")
        );
    }

    #[test]
    fn bundle_surface_binds_manifest_binary_schemas_and_commands() {
        let temporary = tempfile::tempdir().expect("bundle tempdir");
        let plugin = temporary.path().join("reposeiri");
        fs::create_dir_all(plugin.join(".codex-plugin")).expect("manifest directory");
        fs::create_dir_all(plugin.join("skills/reposeiri")).expect("skill directory");
        fs::create_dir_all(plugin.join("scripts")).expect("scripts directory");
        fs::create_dir_all(plugin.join("bin")).expect("binary directory");
        fs::write(
            plugin.join(".codex-plugin/plugin.json"),
            format!(
                "{{\"name\":\"reposeiri\",\"version\":\"{}\",\"skills\":\"./skills/\"}}",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .expect("plugin manifest");
        fs::write(plugin.join("skills/reposeiri/SKILL.md"), "# Skill\n").expect("skill");
        fs::write(plugin.join("scripts/reposeiri-codex.ps1"), "# script\n").expect("PowerShell");
        fs::write(plugin.join("scripts/reposeiri-codex.sh"), "#!/bin/sh\n").expect("shell");
        fs::write(plugin.join("bin/seiri"), b"binary").expect("binary");
        copy_tree(
            &repository_root().expect("repository root").join("schemas"),
            &plugin.join("schemas"),
        )
        .expect("schemas");
        let source = SourceBinding {
            git_head: "test".to_string(),
            worktree_dirty: false,
            source_digest: format!("sha256:{}", "1".repeat(64)),
            cargo_lock_digest: format!("sha256:{}", "2".repeat(64)),
        };
        let manifest = RuntimeManifest {
            schema_version: RUNTIME_MANIFEST_SCHEMA.to_string(),
            bundle_metadata_version: BUNDLE_METADATA_VERSION.to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            target: "test-target".to_string(),
            binary: "bin/seiri".to_string(),
            sha256: sha256_file(&plugin.join("bin/seiri")).expect("binary digest"),
            contract_schema: seiri_core::CONTRACT_SCHEMA_VERSION.to_string(),
            analysis_schema: seiri_core::ANALYSIS_SCHEMA_VERSION.to_string(),
            patch_plan_schema: seiri_core::PATCH_PLAN_SCHEMA_VERSION.to_string(),
            codex_schema: seiri_core::CODEX_SCHEMA_VERSION.to_string(),
            error_schema: seiri_core::ERROR_SCHEMA_VERSION.to_string(),
            completion_schema: seiri_core::COMPLETION_SCHEMA_VERSION.to_string(),
            portable_audit_schema: seiri_core::PORTABLE_AUDIT_SCHEMA_VERSION.to_string(),
            audit_delta_schema: seiri_core::AUDIT_DELTA_SCHEMA_VERSION.to_string(),
            semantic_revisions: seiri_core::SemanticRevisions::default(),
            standalone_smoke: "passed".to_string(),
            source_digest: source.source_digest.clone(),
            cargo_lock_digest: source.cargo_lock_digest.clone(),
            command_set: HOST_COMMAND_SET.iter().map(ToString::to_string).collect(),
            schema_sha256: schema_digests(&plugin.join("schemas")).expect("schema digests"),
        };
        fs::write(
            plugin.join("runtime-manifest.json"),
            serde_json::to_vec_pretty(&manifest).expect("runtime manifest"),
        )
        .expect("runtime manifest");

        validate_bundle_surface(&plugin, &source).expect("valid bundle");
        fs::write(
            plugin.join("schemas/seiri.codex.v2.json"),
            b"{\"tampered\":true}",
        )
        .expect("tamper schema");
        assert!(validate_bundle_surface(&plugin, &source).is_err());
    }
}
