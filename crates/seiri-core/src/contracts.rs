use serde::{Deserialize, Serialize};

pub const ERROR_SCHEMA_VERSION: &str = "seiri.error.v1";
pub const COMPLETION_SCHEMA_VERSION: &str = "seiri.completion.v1";
pub const CONTRACT_SCHEMA_VERSION: &str = "seiri.contract.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    InvalidInput,
    Io,
    Contract,
    Internal,
}

impl ErrorClass {
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::InvalidInput => 3,
            Self::Io => 4,
            Self::Contract => 5,
            Self::Internal => 70,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    pub schema_version: String,
    pub class: ErrorClass,
    pub code: String,
    pub message: String,
}

impl ErrorEnvelope {
    #[must_use]
    pub fn new(class: ErrorClass, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema_version: ERROR_SCHEMA_VERSION.to_string(),
            class,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractManifest {
    pub schema_version: String,
    pub tool_version: String,
    pub analysis_schema: String,
    pub patch_plan_schema: String,
    pub codex_schema: String,
    pub error_schema: String,
    pub completion_schema: String,
    pub compatibility: String,
}

impl ContractManifest {
    #[must_use]
    pub fn current(tool_version: impl Into<String>) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
            tool_version: tool_version.into(),
            analysis_schema: crate::ANALYSIS_SCHEMA_VERSION.to_string(),
            patch_plan_schema: crate::PATCH_PLAN_SCHEMA_VERSION.to_string(),
            codex_schema: crate::CODEX_SCHEMA_VERSION.to_string(),
            error_schema: ERROR_SCHEMA_VERSION.to_string(),
            completion_schema: COMPLETION_SCHEMA_VERSION.to_string(),
            compatibility: "v2-only; v1 inputs, aliases, and silent conversions are rejected"
                .to_string(),
        }
    }
}
