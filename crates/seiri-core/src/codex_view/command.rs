use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexCommand {
    program: String,
    args: Vec<String>,
}

impl CodexCommand {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, CodexCommandError> {
        let program = program.into();
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        validate_command(&program, &args)?;
        Ok(Self { program, args })
    }

    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    #[must_use]
    pub fn render_powershell(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(quote_powershell_argument)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl<'de> Deserialize<'de> for CodexCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireCommand {
            program: String,
            args: Vec<String>,
        }

        let wire = WireCommand::deserialize(deserializer)?;
        Self::new(wire.program, wire.args).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCommandError {
    EmptyProgram,
    ProgramContainsNul,
    ArgumentContainsNul { index: usize },
}

impl Display for CodexCommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProgram => formatter.write_str("Codex command program must not be empty"),
            Self::ProgramContainsNul => {
                formatter.write_str("Codex command program must not contain NUL")
            }
            Self::ArgumentContainsNul { index } => {
                write!(formatter, "Codex command argument {index} contains NUL")
            }
        }
    }
}

impl std::error::Error for CodexCommandError {}

fn validate_command(program: &str, args: &[String]) -> Result<(), CodexCommandError> {
    if program.trim().is_empty() {
        return Err(CodexCommandError::EmptyProgram);
    }
    if program.contains('\0') {
        return Err(CodexCommandError::ProgramContainsNul);
    }
    for (index, argument) in args.iter().enumerate() {
        if argument.contains('\0') {
            return Err(CodexCommandError::ArgumentContainsNul { index });
        }
    }
    Ok(())
}

fn quote_powershell_argument(value: &str) -> String {
    if !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b'/' | b'\\' | b':' | b'=')
        })
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}
