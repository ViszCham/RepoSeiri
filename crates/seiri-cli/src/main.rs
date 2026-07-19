#![forbid(unsafe_code)]

use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use seiri_core::ProfileKind;
use std::path::PathBuf;
use std::process::ExitCode;

mod codex;

use seiri_report::CodexQueryKind;

#[derive(Debug, Parser)]
#[command(name = "seiri")]
#[command(about = "Bounded local repository audit and dry-run planning")]
#[command(
    long_about = "RepoSeiri inspects repository routes, documents, GitHub-local configuration, and Git-local structure from bounded local evidence. Standard audits do not write files, initiate network access, or perform GitHub operations."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the v2 wire contract and semantic revisions.
    Contract {
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Build one canonical local analysis and render it.
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
    /// Produce an existing-target-only dry-run patch plan.
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
    /// Compare two portable audits without exposing source bodies.
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
    /// Lint visible Markdown prose while excluding code and comments.
    LintWording {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Aggregate a bounded public calibration dataset.
    Calibrate {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Print the built-in pattern registry.
    Patterns {
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Project one canonical analysis through one of ten Codex queries.
    Codex {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        #[arg(long, value_enum, default_value_t = ScopeArg::Repository)]
        scope: ScopeArg,
        #[arg(
            long,
            default_value = "summary",
            value_name = "QUERY",
            help = "summary|routes|evidence|documents|governance|patches|linter|actions|remote|pr-body",
            value_parser = parse_codex_query
        )]
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
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return match error.print() {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::from(seiri_core::ErrorClass::Io.exit_code()),
            };
        }
        Err(error) => {
            return emit_error(CliError {
                class: seiri_core::ErrorClass::InvalidInput,
                code: "cli_parse_failed",
                message: error.to_string(),
            });
        }
    };
    match run(cli) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => emit_error(error),
    }
}

fn emit_error(error: CliError) -> ExitCode {
    let class = error.class;
    let envelope = seiri_core::ErrorEnvelope::new(class, error.code, error.message);
    match serde_json::to_string(&envelope) {
        Ok(rendered) => eprintln!("{rendered}"),
        Err(_) => eprintln!(
            "{{\"schema_version\":\"seiri.error.v1\",\"class\":\"internal\",\"code\":\"error_render_failed\",\"message\":\"failed to render typed error\"}}"
        ),
    }
    ExitCode::from(class.exit_code())
}

#[derive(Debug)]
struct CliError {
    class: seiri_core::ErrorClass,
    code: &'static str,
    message: String,
}

impl From<seiri_report::AuditError> for CliError {
    fn from(error: seiri_report::AuditError) -> Self {
        let class = error.error_class();
        let code = error.error_code();
        Self {
            class,
            code,
            message: error.to_string(),
        }
    }
}

fn run(cli: Cli) -> Result<String, CliError> {
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
