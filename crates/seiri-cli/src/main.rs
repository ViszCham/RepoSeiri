use clap::{Parser, Subcommand, ValueEnum};
use seiri_core::ProfileKind;
use std::path::PathBuf;
use std::process::ExitCode;

mod codex;

use codex::{CodexError, CodexSchema, CodexView};
use seiri_report::CodexNativeV3QueryKind;

#[derive(Debug, Parser)]
#[command(name = "seiri")]
#[command(about = "RepoSeiri repository audit CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Audit {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long)]
        calibration_priors: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = ScopeArg::Repository)]
        scope: ScopeArg,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    Plan {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long)]
        calibration_priors: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    PlanV5 {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long)]
        calibration_priors: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = ScopeArg::Repository)]
        scope: ScopeArg,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    Diff {
        #[arg(long)]
        before: PathBuf,
        #[arg(long)]
        after: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long)]
        before_calibration_priors: Option<PathBuf>,
        #[arg(long)]
        after_calibration_priors: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = ScopeArg::Repository)]
        before_scope: ScopeArg,
        #[arg(long, value_enum, default_value_t = ScopeArg::Repository)]
        after_scope: ScopeArg,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    LintWording {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    Calibrate {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    Patterns {
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    Codex {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        #[arg(long, value_enum, default_value_t = CodexView::Context)]
        view: CodexView,
        #[arg(long, value_enum, default_value_t = CodexSchema::CompatibilityV1)]
        schema: CodexSchema,
        #[arg(long, default_value = "summary", value_parser = parse_codex_query)]
        query: CodexNativeV3QueryKind,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScopeArg {
    Repository,
    Workspace,
    Subtree,
}

impl From<ScopeArg> for seiri_core::AnalysisScope {
    fn from(value: ScopeArg) -> Self {
        match value {
            ScopeArg::Repository => Self::Repository,
            ScopeArg::Workspace => Self::Workspace,
            ScopeArg::Subtree => Self::Subtree,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("seiri: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, CodexError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Audit {
            path,
            profile,
            calibration_priors,
            scope,
            format,
        } => {
            let snapshot = match calibration_priors {
                Some(priors) => seiri_report::audit_repository_with_local_calibration_and_scope(
                    path,
                    profile,
                    priors,
                    scope.into(),
                )?,
                None => seiri_report::audit_repository_with_scope(path, profile, scope.into())?,
            };
            match format {
                OutputFormat::Json => Ok(seiri_report::to_json(&snapshot)?),
                OutputFormat::Markdown => Ok(seiri_report::to_markdown(&snapshot)),
            }
        }
        Command::Plan {
            path,
            profile,
            calibration_priors,
            format,
        } => {
            let plan = match calibration_priors {
                Some(priors) => {
                    seiri_report::plan_repository_with_local_calibration(path, profile, priors)?
                }
                None => seiri_report::plan_repository_with_profile(path, profile)?,
            };
            match format {
                OutputFormat::Json => Ok(seiri_report::plan_to_json(&plan)?),
                OutputFormat::Markdown => Ok(seiri_report::plan_to_markdown(&plan)),
            }
        }
        Command::PlanV5 {
            path,
            profile,
            calibration_priors,
            scope,
            format,
        } => {
            let plan = match calibration_priors {
                Some(priors) => seiri_report::plan_repository_v5_with_local_calibration(
                    path,
                    profile,
                    priors,
                    scope.into(),
                )?,
                None => seiri_report::plan_repository_v5(path, profile, scope.into())?,
            };
            match format {
                OutputFormat::Json => Ok(seiri_report::planner_v5_to_json(&plan)?),
                OutputFormat::Markdown => Ok(seiri_report::planner_v5_to_markdown(&plan)),
            }
        }
        Command::Diff {
            before,
            after,
            profile,
            before_calibration_priors,
            after_calibration_priors,
            before_scope,
            after_scope,
            format,
        } => {
            let before_snapshot = match before_calibration_priors {
                Some(priors) => seiri_report::audit_repository_with_local_calibration_and_scope(
                    before,
                    profile,
                    priors,
                    before_scope.into(),
                )?,
                None => {
                    seiri_report::audit_repository_with_scope(before, profile, before_scope.into())?
                }
            };
            let after_snapshot = match after_calibration_priors {
                Some(priors) => seiri_report::audit_repository_with_local_calibration_and_scope(
                    after,
                    profile,
                    priors,
                    after_scope.into(),
                )?,
                None => {
                    seiri_report::audit_repository_with_scope(after, profile, after_scope.into())?
                }
            };
            let delta = seiri_report::diff_snapshots(&before_snapshot, &after_snapshot)?;
            match format {
                OutputFormat::Json => Ok(seiri_report::audit_delta_to_json(&delta)?),
                OutputFormat::Markdown => Ok(seiri_report::audit_delta_to_markdown(&delta)),
            }
        }
        Command::LintWording {
            path,
            profile,
            format,
        } => {
            let report = seiri_report::lint_wording_repository_with_profile(path, profile)?;
            match format {
                OutputFormat::Json => Ok(seiri_report::wording_lint_to_json(&report)?),
                OutputFormat::Markdown => Ok(seiri_report::wording_lint_to_markdown(&report)),
            }
        }
        Command::Calibrate { input, format } => {
            let run = seiri_report::calibrate_dataset_path(input)?;
            match format {
                OutputFormat::Json => Ok(seiri_report::calibration_to_json(&run)?),
                OutputFormat::Markdown => Ok(seiri_report::calibration_to_markdown(&run)),
            }
        }
        Command::Patterns { format } => match format {
            OutputFormat::Json => Ok(seiri_report::pattern_registry_to_json()?),
            OutputFormat::Markdown => Ok(seiri_report::pattern_registry_to_markdown()),
        },
        Command::Codex {
            path,
            profile,
            format,
            view,
            schema,
            query,
        } => codex::render(path, profile, format, view, schema, query),
    }
}

fn parse_profile(value: &str) -> Result<ProfileKind, String> {
    value.parse::<ProfileKind>().map_err(|error| {
        format!(
            "{error}; expected one of: common, library, cli, infra, product, runtime, docs, tutorial, ml, research, template"
        )
    })
}

fn parse_codex_query(value: &str) -> Result<CodexNativeV3QueryKind, String> {
    value
        .parse::<CodexNativeV3QueryKind>()
        .map_err(|error| error.to_string())
}
