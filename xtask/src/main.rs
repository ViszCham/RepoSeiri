#![forbid(unsafe_code)]

mod bundle;
mod completion;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<OsString>) -> Result<ExitCode, String> {
    let Some(command) = args.first().and_then(|value| value.to_str()) else {
        return Err(usage());
    };
    match command {
        "completion" => completion::run(&args[1..]),
        "bundle" => bundle::run(&args[1..]),
        _ => Err(usage()),
    }
}

fn repository_root() -> Result<PathBuf, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "xtask manifest has no repository parent".to_string())?
        .to_path_buf();
    if !root.join("Cargo.toml").is_file() || !root.join("plugins/reposeiri").is_dir() {
        return Err("repository root contract is not satisfied".to_string());
    }
    Ok(root)
}

fn usage() -> String {
    "usage: cargo run -p xtask -- completion --format json [--host-evidence <directory>] | bundle --target <triple> --binary <path> --output <new-directory>".to_string()
}
