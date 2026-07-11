use super::OutputFormat;
use seiri_core::{AnalysisScope, ProfileKind};
use seiri_report::CodexQueryKind;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug)]
pub(super) struct CodexError(seiri_report::AuditError);

impl Display for CodexError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for CodexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl From<seiri_report::AuditError> for CodexError {
    fn from(value: seiri_report::AuditError) -> Self {
        Self(value)
    }
}

pub(super) fn render(
    path: PathBuf,
    profile: ProfileKind,
    scope: AnalysisScope,
    format: OutputFormat,
    query: CodexQueryKind,
) -> Result<String, CodexError> {
    match format {
        OutputFormat::Json => Ok(seiri_report::codex_query_repository_to_json(
            path, profile, scope, query,
        )?),
        OutputFormat::Markdown => Ok(seiri_report::codex_query_repository_to_markdown(
            path, profile, scope, query,
        )?),
    }
}
