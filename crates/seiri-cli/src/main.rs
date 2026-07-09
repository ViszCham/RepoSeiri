use clap::{Parser, Subcommand, ValueEnum};
use seiri_core::ProfileKind;
use std::path::PathBuf;
use std::process::ExitCode;

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
    Calibrate {
        #[arg(long)]
        input: PathBuf,
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
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CodexView {
    Context,
    PrBody,
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
        Command::Calibrate { input, format } => {
            let run = seiri_report::calibrate_dataset_path(input)?;
            match format {
                OutputFormat::Json => seiri_report::calibration_to_json(&run),
                OutputFormat::Markdown => Ok(seiri_report::calibration_to_markdown(&run)),
            }
        }
        Command::Codex {
            path,
            profile,
            format,
            view,
        } => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            match (view, format) {
                (CodexView::Context, OutputFormat::Json) => seiri_report::codex_to_json(&context),
                (CodexView::Context, OutputFormat::Markdown) => {
                    Ok(seiri_report::codex_to_markdown(&context))
                }
                (CodexView::PrBody, OutputFormat::Json) => {
                    seiri_report::codex_pr_draft_to_json(&context)
                }
                (CodexView::PrBody, OutputFormat::Markdown) => {
                    Ok(seiri_report::codex_pr_body_to_markdown(&context))
                }
            }
        }
    }
}

fn parse_profile(value: &str) -> Result<ProfileKind, String> {
    value.parse::<ProfileKind>().map_err(|error| {
        format!(
            "{error}; expected one of: common, library, cli, infra, docs, tutorial, research, template"
        )
    })
}
