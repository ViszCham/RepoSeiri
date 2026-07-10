use super::OutputFormat;
use clap::ValueEnum;
use seiri_core::{CodexQueryKind, ProfileKind};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum CodexView {
    Context,
    PrBody,
    Query,
    Linter,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum CodexSchema {
    CompatibilityV1,
    NativeV2,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum CodexQuery {
    Summary,
    Routes,
    Patches,
    Linter,
    Actions,
}

impl From<CodexQuery> for CodexQueryKind {
    fn from(value: CodexQuery) -> Self {
        match value {
            CodexQuery::Summary => Self::Summary,
            CodexQuery::Routes => Self::Routes,
            CodexQuery::Patches => Self::Patches,
            CodexQuery::Linter => Self::Linter,
            CodexQuery::Actions => Self::Actions,
        }
    }
}

pub(super) fn render(
    path: PathBuf,
    profile: ProfileKind,
    format: OutputFormat,
    view: CodexView,
    schema: CodexSchema,
    query: CodexQuery,
) -> Result<String, seiri_report::AuditError> {
    match (view, schema, format) {
        (CodexView::Context, CodexSchema::CompatibilityV1, OutputFormat::Json) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            seiri_report::codex_to_json(&context)
        }
        (CodexView::Context, CodexSchema::CompatibilityV1, OutputFormat::Markdown) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_to_markdown(&context))
        }
        (CodexView::Context, CodexSchema::NativeV2, OutputFormat::Json) => {
            let context = seiri_report::codex_native_repository_with_profile(path, profile)?;
            seiri_report::codex_native_to_json(&context)
        }
        (CodexView::Context, CodexSchema::NativeV2, OutputFormat::Markdown) => {
            let context = seiri_report::codex_native_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_native_to_markdown(&context))
        }
        (CodexView::PrBody, _, OutputFormat::Json) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            seiri_report::codex_pr_draft_to_json(&context)
        }
        (CodexView::PrBody, _, OutputFormat::Markdown) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_pr_body_to_markdown(&context))
        }
        (CodexView::Query, _, OutputFormat::Json) => {
            let view =
                seiri_report::codex_query_repository_with_profile(path, profile, query.into())?;
            seiri_report::codex_query_to_json(&view)
        }
        (CodexView::Query, _, OutputFormat::Markdown) => {
            let view =
                seiri_report::codex_query_repository_with_profile(path, profile, query.into())?;
            Ok(seiri_report::codex_query_to_markdown(&view))
        }
        (CodexView::Linter, _, OutputFormat::Json) => {
            let context = seiri_report::codex_linter_repository_with_profile(path, profile)?;
            seiri_report::codex_linter_context_to_json(&context)
        }
        (CodexView::Linter, _, OutputFormat::Markdown) => {
            let context = seiri_report::codex_linter_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_linter_context_to_markdown(&context))
        }
    }
}
