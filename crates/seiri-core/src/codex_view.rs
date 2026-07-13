mod command;

use serde::{Deserialize, Serialize};

pub use command::{CodexCommand, CodexCommandError};

pub const CODEX_SCHEMA_VERSION: &str = "seiri.codex.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexAction {
    pub id: String,
    pub label: String,
    pub command: CodexCommand,
    pub mutates_files: bool,
    pub requires_confirmation: bool,
    pub detail: String,
}
