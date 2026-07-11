use super::OutputFormat;
use seiri_core::ProfileKind;
use seiri_report::CodexNativeV3QueryKind;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

mod request;

use request::{validate_request, CodexRequestError};
pub(super) use request::{CodexSchema, CodexView};

#[derive(Debug)]
pub(super) enum CodexError {
    Audit(seiri_report::AuditError),
    Request(CodexRequestError),
}

impl Display for CodexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Audit(error) => Display::fmt(error, f),
            Self::Request(error) => Display::fmt(error, f),
        }
    }
}

impl std::error::Error for CodexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Audit(error) => Some(error),
            Self::Request(error) => Some(error),
        }
    }
}

impl From<seiri_report::AuditError> for CodexError {
    fn from(value: seiri_report::AuditError) -> Self {
        Self::Audit(value)
    }
}

impl From<CodexRequestError> for CodexError {
    fn from(value: CodexRequestError) -> Self {
        Self::Request(value)
    }
}

pub(super) fn render(
    path: PathBuf,
    profile: ProfileKind,
    format: OutputFormat,
    view: CodexView,
    schema: CodexSchema,
    query: CodexNativeV3QueryKind,
) -> Result<String, CodexError> {
    validate_request(schema, view, query)?;

    match (view, schema, format) {
        (CodexView::Context, CodexSchema::CompatibilityV1, OutputFormat::Json) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_to_json(&context)?)
        }
        (CodexView::Context, CodexSchema::CompatibilityV1, OutputFormat::Markdown) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_to_markdown(&context))
        }
        (CodexView::Context, CodexSchema::NativeV2, OutputFormat::Json) => {
            let context = seiri_report::codex_native_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_native_to_json(&context)?)
        }
        (CodexView::Context, CodexSchema::NativeV2, OutputFormat::Markdown) => {
            let context = seiri_report::codex_native_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_native_to_markdown(&context))
        }
        (CodexView::Context, CodexSchema::NativeV3, format) => {
            render_native_v3(path, profile, format, CodexNativeV3QueryKind::Summary)
        }
        (CodexView::PrBody, CodexSchema::CompatibilityV1, OutputFormat::Json) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_pr_draft_to_json(&context)?)
        }
        (CodexView::PrBody, CodexSchema::CompatibilityV1, OutputFormat::Markdown) => {
            let context = seiri_report::codex_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_pr_body_to_markdown(&context))
        }
        (CodexView::Query, CodexSchema::NativeV3, format) => {
            render_native_v3(path, profile, format, query)
        }
        (CodexView::Query, _, OutputFormat::Json) => {
            let legacy = query
                .compatibility_kind()
                .expect("request validation guarantees a compatibility query");
            let view = seiri_report::codex_query_repository_with_profile(path, profile, legacy)?;
            Ok(seiri_report::codex_query_to_json(&view)?)
        }
        (CodexView::Query, _, OutputFormat::Markdown) => {
            let legacy = query
                .compatibility_kind()
                .expect("request validation guarantees a compatibility query");
            let view = seiri_report::codex_query_repository_with_profile(path, profile, legacy)?;
            Ok(seiri_report::codex_query_to_markdown(&view))
        }
        (CodexView::Linter, CodexSchema::NativeV3, format) => {
            render_native_v3(path, profile, format, CodexNativeV3QueryKind::Linter)
        }
        (CodexView::Linter, _, OutputFormat::Json) => {
            let context = seiri_report::codex_linter_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_linter_context_to_json(&context)?)
        }
        (CodexView::Linter, _, OutputFormat::Markdown) => {
            let context = seiri_report::codex_linter_repository_with_profile(path, profile)?;
            Ok(seiri_report::codex_linter_context_to_markdown(&context))
        }
        (CodexView::PrBody, _, _) => unreachable!("request validation rejects this schema/view"),
    }
}

fn render_native_v3(
    path: PathBuf,
    profile: ProfileKind,
    format: OutputFormat,
    query: CodexNativeV3QueryKind,
) -> Result<String, CodexError> {
    match format {
        OutputFormat::Json => Ok(seiri_report::codex_native_v3_query_repository_to_json(
            path, profile, query,
        )?),
        OutputFormat::Markdown => Ok(seiri_report::codex_native_v3_query_repository_to_markdown(
            path, profile, query,
        )?),
    }
}
