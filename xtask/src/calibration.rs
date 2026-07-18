use crate::repository_root;
use std::ffi::OsString;
use std::process::ExitCode;

pub fn run(args: &[OsString]) -> Result<ExitCode, String> {
    if option(args, "--format")? != "json" {
        return Err("calibration-holdout supports only '--format json'".to_string());
    }
    let root = repository_root()?;
    let report = evaluate(&root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn evaluate(
    root: &std::path::Path,
) -> Result<seiri_report::HoldoutCalibrationReport, String> {
    seiri_report::evaluate_public_holdout(
        root.join("fixtures/calibration-holdout-corpus.v1.json"),
        root.join("fixtures"),
    )
    .map_err(|error| error.to_string())
}

fn option<'a>(args: &'a [OsString], name: &str) -> Result<&'a str, String> {
    let index = args
        .iter()
        .position(|value| value == name)
        .ok_or_else(|| format!("missing value for {name}"))?;
    args.get(index + 1)
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("missing value for {name}"))
}
