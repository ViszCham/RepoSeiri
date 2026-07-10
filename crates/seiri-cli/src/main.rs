use clap::{Parser, Subcommand, ValueEnum};
use seiri_core::ProfileKind;
use std::path::PathBuf;
use std::process::ExitCode;

mod codex;

use codex::{CodexQuery, CodexSchema, CodexView};

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
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    Plan {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "common", value_parser = parse_profile)]
        profile: ProfileKind,
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
        #[arg(long, value_enum, default_value_t = CodexQuery::Summary)]
        query: CodexQuery,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum OutputFormat {
    Json,
    Markdown,
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

fn run() -> Result<String, seiri_report::AuditError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Audit {
            path,
            profile,
            format,
        } => {
            let snapshot = seiri_report::audit_repository_with_profile(path, profile)?;
            match format {
                OutputFormat::Json => seiri_report::to_json(&snapshot),
                OutputFormat::Markdown => Ok(seiri_report::to_markdown(&snapshot)),
            }
        }
        Command::Plan {
            path,
            profile,
            format,
        } => {
            let plan = seiri_report::plan_repository_with_profile(path, profile)?;
            match format {
                OutputFormat::Json => seiri_report::plan_to_json(&plan),
                OutputFormat::Markdown => Ok(seiri_report::plan_to_markdown(&plan)),
            }
        }
        Command::LintWording {
            path,
            profile,
            format,
        } => {
            let report = seiri_report::lint_wording_repository_with_profile(path, profile)?;
            match format {
                OutputFormat::Json => seiri_report::wording_lint_to_json(&report),
                OutputFormat::Markdown => Ok(seiri_report::wording_lint_to_markdown(&report)),
            }
        }
        Command::Calibrate { input, format } => {
            let run = seiri_report::calibrate_dataset_path(input)?;
            match format {
                OutputFormat::Json => seiri_report::calibration_to_json(&run),
                OutputFormat::Markdown => Ok(seiri_report::calibration_to_markdown(&run)),
            }
        }
        Command::Patterns { format } => match format {
            OutputFormat::Json => seiri_report::pattern_registry_to_json(),
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
