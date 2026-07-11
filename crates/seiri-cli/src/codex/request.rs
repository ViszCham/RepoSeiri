use clap::ValueEnum;
use seiri_report::CodexNativeV3QueryKind;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CodexView {
    Context,
    PrBody,
    Query,
    Linter,
}

impl CodexView {
    const fn slug(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::PrBody => "pr-body",
            Self::Query => "query",
            Self::Linter => "linter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CodexSchema {
    CompatibilityV1,
    NativeV2,
    NativeV3,
}

impl CodexSchema {
    const fn slug(self) -> &'static str {
        match self {
            Self::CompatibilityV1 => "compatibility-v1",
            Self::NativeV2 => "native-v2",
            Self::NativeV3 => "native-v3",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexRequestError {
    schema: CodexSchema,
    view: CodexView,
    query: CodexNativeV3QueryKind,
    supported: &'static str,
}

impl Display for CodexRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported Codex request: schema `{}`, view `{}`, query `{}`; supported: {}",
            self.schema.slug(),
            self.view.slug(),
            self.query.slug(),
            self.supported
        )
    }
}

impl std::error::Error for CodexRequestError {}

pub(super) fn validate_request(
    schema: CodexSchema,
    view: CodexView,
    query: CodexNativeV3QueryKind,
) -> Result<(), CodexRequestError> {
    let supported = match (schema, view) {
        (_, CodexView::Context | CodexView::PrBody | CodexView::Linter)
            if query != CodexNativeV3QueryKind::Summary =>
        {
            Some("query `summary` for context/pr-body/linter views")
        }
        (CodexSchema::NativeV2 | CodexSchema::NativeV3, CodexView::PrBody) => {
            Some("schema `compatibility-v1` for view `pr-body`")
        }
        (CodexSchema::CompatibilityV1 | CodexSchema::NativeV2, CodexView::Query)
            if query.compatibility_kind().is_none() =>
        {
            Some("queries `summary`, `routes`, `patches`, `linter`, `actions`")
        }
        _ => None,
    };
    supported.map_or(Ok(()), |supported| {
        Err(CodexRequestError {
            schema,
            view,
            query,
            supported,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_matrix_rejects_silent_fallbacks() {
        assert!(validate_request(
            CodexSchema::NativeV2,
            CodexView::Query,
            CodexNativeV3QueryKind::Evidence,
        )
        .is_err());
        assert!(validate_request(
            CodexSchema::NativeV3,
            CodexView::PrBody,
            CodexNativeV3QueryKind::Summary,
        )
        .is_err());
        assert!(validate_request(
            CodexSchema::CompatibilityV1,
            CodexView::PrBody,
            CodexNativeV3QueryKind::Routes,
        )
        .is_err());
        assert!(validate_request(
            CodexSchema::NativeV3,
            CodexView::Query,
            CodexNativeV3QueryKind::Remote,
        )
        .is_ok());
    }
}
