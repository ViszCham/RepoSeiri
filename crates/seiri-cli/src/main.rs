#![forbid(unsafe_code)]

use clap::{Parser, Subcommand, ValueEnum};
use seiri_core::ProfileKind;
use std::path::PathBuf;
use std::process::ExitCode;

mod codex;

use seiri_report::CodexQueryKind;

#[derive(Debug, Parser)]
#[command(name = "seiri")]
#[command(about = "RepoSeiri repository audit CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Contract {
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
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
        #[arg(long, value_enum, default_value_t = ScopeArg::Repository)]
        scope: ScopeArg,
        #[arg(long, default_value = "summary", value_parser = parse_codex_query)]
        query: CodexQueryKind,
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
            let envelope = seiri_core::ErrorEnvelope::new(error.class, error.code, error.message);
            match serde_json::to_string(&envelope) {
                Ok(rendered) => eprintln!("{rendered}"),
                Err(_) => eprintln!(
                    "{{\"schema_version\":\"seiri.error.v1\",\"class\":\"internal\",\"code\":\"error_render_failed\",\"message\":\"failed to render typed error\"}}"
                ),
            }
            ExitCode::from(error.class.exit_code())
        }
    }
}

#[derive(Debug)]
struct CliError {
    class: seiri_core::ErrorClass,
    code: &'static str,
    message: String,
}

impl From<seiri_report::AuditError> for CliError {
    fn from(error: seiri_report::AuditError) -> Self {
        let class = match &error {
            seiri_report::AuditError::Fs(_)
            | seiri_report::AuditError::Markdown(_)
            | seiri_report::AuditError::LocalPrior(_)
            | seiri_report::AuditError::Io { .. } => seiri_core::ErrorClass::Io,
            seiri_report::AuditError::Calibration(_)
            | seiri_report::AuditError::DocumentIndex(_)
            | seiri_report::AuditError::GithubLocal(_)
            | seiri_report::AuditError::Coverage(_)
            | seiri_report::AuditError::RouteAssessment(_)
            | seiri_report::AuditError::DocumentConsistency(_)
            | seiri_report::AuditError::Delta(_) => seiri_core::ErrorClass::InvalidInput,
            seiri_report::AuditError::EvidenceKernel(_)
            | seiri_report::AuditError::GitLocal(_)
            | seiri_report::AuditError::Json(_) => seiri_core::ErrorClass::Internal,
        };
        Self {
            class,
            code: "audit_failed",
            message: error.to_string(),
        }
    }
}

fn run() -> Result<String, CliError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Contract { format } => {
            let contract = seiri_core::ContractManifest::current(env!("CARGO_PKG_VERSION"));
            match format {
                OutputFormat::Json => serde_json::to_string_pretty(&contract).map_err(|error| {
                    CliError {
                        class: seiri_core::ErrorClass::Internal,
                        code: "contract_render_failed",
                        message: error.to_string(),
                    }
                }),
                OutputFormat::Markdown => Ok(format!(
                    "# RepoSeiri Contract\n\n- Tool: `{}`\n- Analysis: `{}`\n- Patch plan: `{}`\n- Codex: `{}`\n- Error: `{}`\n- Completion: `{}`\n- Compatibility: {}",
                    contract.tool_version,
                    contract.analysis_schema,
                    contract.patch_plan_schema,
                    contract.codex_schema,
                    contract.error_schema,
                    contract.completion_schema,
                    contract.compatibility
                )),
            }
        }
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
            scope,
            format,
        } => {
            let plan = match calibration_priors {
                Some(priors) => seiri_report::plan_repository_with_local_calibration_and_scope(
                    path,
                    profile,
                    priors,
                    scope.into(),
                )?,
                None => seiri_report::plan_repository_with_scope(path, profile, scope.into())?,
            };
            match format {
                OutputFormat::Json => Ok(seiri_report::plan_to_json(&plan)?),
                OutputFormat::Markdown => Ok(seiri_report::plan_to_markdown(&plan)),
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
            scope,
            query,
        } => codex::render(path, profile, scope.into(), format, query),
    }
}

fn parse_profile(value: &str) -> Result<ProfileKind, String> {
    value.parse::<ProfileKind>().map_err(|error| {
        format!(
            "{error}; expected one of: common, library, cli, infra, product, runtime, docs, tutorial, ml, research, template"
        )
    })
}

fn parse_codex_query(value: &str) -> Result<CodexQueryKind, String> {
    value
        .parse::<CodexQueryKind>()
        .map_err(|error| error.to_string())
}
