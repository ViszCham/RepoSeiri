use crate::repository_root;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeManifest {
    schema_version: String,
    tool_version: String,
    target: String,
    binary: String,
    sha256: String,
    codex_schema: String,
    error_schema: String,
    standalone_smoke: String,
}

#[derive(Debug, Serialize)]
pub struct HostEvidenceRecord {
    pub target: &'static str,
    pub status: HostEvidenceStatus,
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
    let plugin_output = output.join("reposeiri");
    copy_tree(&root.join("plugins/reposeiri"), &plugin_output)?;
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
    smoke_bundle(&plugin_output, &bundled_binary, target)?;
    let manifest = RuntimeManifest {
        schema_version: "reposeiri.runtime-manifest.v1".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        target: target.to_string(),
        binary: format!("bin/{binary_name}"),
        sha256: sha256_file(&bundled_binary)?,
        codex_schema: seiri_core::CODEX_SCHEMA_VERSION.to_string(),
        error_schema: seiri_core::ERROR_SCHEMA_VERSION.to_string(),
        standalone_smoke: "passed".to_string(),
    };
    fs::write(
        plugin_output.join("runtime-manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&manifest).map_err(|error| error.to_string())?
    );
    Ok(ExitCode::SUCCESS)
}

pub fn validate_required_hosts(directory: Option<&Path>) -> Vec<HostEvidenceRecord> {
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
            let status = match matching {
                None => HostEvidenceStatus::Missing,
                Some((path, manifest)) => {
                    let binary = path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(&manifest.binary);
                    if manifest.schema_version == "reposeiri.runtime-manifest.v1"
                        && manifest.tool_version == env!("CARGO_PKG_VERSION")
                        && manifest.codex_schema == seiri_core::CODEX_SCHEMA_VERSION
                        && manifest.error_schema == seiri_core::ERROR_SCHEMA_VERSION
                        && manifest.standalone_smoke == "passed"
                        && sha256_file(&binary).ok().as_deref() == Some(manifest.sha256.as_str())
                    {
                        HostEvidenceStatus::Passed
                    } else {
                        HostEvidenceStatus::Invalid
                    }
                }
            };
            HostEvidenceRecord { target, status }
        })
        .collect()
}

fn smoke_bundle(plugin_root: &Path, binary: &Path, target: &str) -> Result<(), String> {
    let current = std::env::current_dir().map_err(|error| error.to_string())?;
    let plugin_root = absolute_from(&current, plugin_root);
    let binary = absolute_from(&current, binary);
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
    let mut command = if target.contains("windows") {
        let mut command = Command::new("powershell");
        command.args([
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
        ]);
        command
    } else {
        let mut command = Command::new("sh");
        command
            .arg(plugin_root.join("scripts/reposeiri-codex.sh"))
            .args(["--path", ".", "--query", "summary", "--format", "json"]);
        command
    };
    let output = command
        .current_dir(&smoke_root)
        .env("REPOSEIRI_BIN", binary)
        .output()
        .map_err(|error| error.to_string())?;
    fs::remove_dir_all(&smoke_root).map_err(|error| error.to_string())?;
    if !output.status.success()
        || !String::from_utf8_lossy(&output.stdout).contains(seiri_core::CODEX_SCHEMA_VERSION)
    {
        return Err("bundled launcher smoke failed".to_string());
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            return Err("plugin bundles do not follow symbolic links".to_string());
        }
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
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

    #[test]
    fn missing_host_evidence_fails_closed() {
        let hosts = validate_required_hosts(None);
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
}
