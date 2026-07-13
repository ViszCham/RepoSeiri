use super::OutputFormat;
use crate::CliError;
use seiri_core::{AnalysisScope, ProfileKind};
use seiri_report::CodexQueryKind;
use std::path::PathBuf;

pub(super) fn render(
    path: PathBuf,
    profile: ProfileKind,
    scope: AnalysisScope,
    format: OutputFormat,
    query: CodexQueryKind,
) -> Result<String, CliError> {
    match format {
        OutputFormat::Json => Ok(seiri_report::codex_query_repository_to_json(
            path, profile, scope, query,
        )?),
        OutputFormat::Markdown => Ok(seiri_report::codex_query_repository_to_markdown(
            path, profile, scope, query,
        )?),
    }
}
