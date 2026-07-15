mod command;

use serde::{Deserialize, Serialize};

pub use command::{CodexCommand, CodexCommandError};

pub const CODEX_SCHEMA_VERSION: &str = "seiri.codex.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexAction {
    pub id: String,
    pub label: String,
    pub command: CodexCommand,
    #[serde(default)]
    pub runtime: CodexRuntimeRequirement,
    pub mutates_files: bool,
    pub requires_confirmation: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexExecutableRole {
    RepoSeiriCli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexRuntimeResolver {
    ConfiguredBinary,
    BundleLocalBinary,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexRuntimeRequirement {
    pub executable: CodexExecutableRole,
    pub resolution: Vec<CodexRuntimeResolver>,
}

impl Default for CodexRuntimeRequirement {
    fn default() -> Self {
        Self {
            executable: CodexExecutableRole::RepoSeiriCli,
            resolution: vec![
                CodexRuntimeResolver::ConfiguredBinary,
                CodexRuntimeResolver::BundleLocalBinary,
                CodexRuntimeResolver::Path,
            ],
        }
    }
}
