use crate::{PatternFixtureKind, PredicateProgramError};
use seiri_core::PatternGroup;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum PatternPackLoadError {
    Io(std::io::ErrorKind),
    Json {
        line: usize,
        column: usize,
    },
    SourceTooLarge,
    SymlinkSource,
    MissingPackDirectory,
    UnsupportedSchema,
    InvalidIdentifier,
    InvalidDefinitionCount,
    InvalidFixtureCount,
    InvalidExpectationCount,
    InvalidFixturePath,
    InvalidScanBudget,
    DuplicateDefinition,
    DuplicateFixture,
    InvalidPredicate(PredicateProgramError),
    AutomaticAdoption,
    MissingAdoptionBoundary,
    InvalidBoundaries,
    UnknownPattern,
    FixtureGroupMismatch,
    PartialExpectsAbsence,
    InvalidEvidenceExpectation,
    InvalidRange,
    MalformedWithoutDiagnostic,
    MissingGroupDefinition(PatternGroup),
    MissingFixtureClass {
        group: PatternGroup,
        kind: PatternFixtureKind,
    },
    FixtureEscape,
    SymlinkEscape,
    FixtureDepthExceeded,
    FixtureEntriesExceeded,
    FixtureFileTooLarge,
    FixtureTotalTooLarge,
    Fingerprint,
}

impl Display for PatternPackLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(kind) => write!(
                formatter,
                "failed to read executable pattern pack: {kind:?}"
            ),
            Self::Json { line, column } => write!(
                formatter,
                "failed to parse executable pattern pack at line {line}, column {column}"
            ),
            Self::SourceTooLarge => formatter.write_str("executable pattern pack is too large"),
            Self::SymlinkSource => {
                formatter.write_str("executable pattern pack source is a symlink")
            }
            Self::MissingPackDirectory => {
                formatter.write_str("executable pattern pack has no parent directory")
            }
            Self::UnsupportedSchema => {
                formatter.write_str("executable pattern pack schema is unsupported")
            }
            Self::InvalidIdentifier => {
                formatter.write_str("executable pattern pack contains an invalid identifier")
            }
            Self::InvalidDefinitionCount => {
                formatter.write_str("executable pattern pack definition count is invalid")
            }
            Self::InvalidFixtureCount => {
                formatter.write_str("executable pattern pack fixture count is invalid")
            }
            Self::InvalidExpectationCount => {
                formatter.write_str("executable fixture expectation count is invalid")
            }
            Self::InvalidFixturePath => {
                formatter.write_str("executable fixture path is not a bounded relative path")
            }
            Self::InvalidScanBudget => {
                formatter.write_str("executable fixture scan budget is invalid")
            }
            Self::DuplicateDefinition => {
                formatter.write_str("executable pattern pack contains a duplicate definition")
            }
            Self::DuplicateFixture => {
                formatter.write_str("executable pattern pack contains a duplicate fixture")
            }
            Self::InvalidPredicate(error) => write!(
                formatter,
                "executable pattern predicate is invalid: {error}"
            ),
            Self::AutomaticAdoption => {
                formatter.write_str("data-only patterns must remain candidates")
            }
            Self::MissingAdoptionBoundary => {
                formatter.write_str("data-only pattern is missing adoption boundaries")
            }
            Self::InvalidBoundaries => {
                formatter.write_str("data-only pattern boundaries are invalid")
            }
            Self::UnknownPattern => {
                formatter.write_str("executable fixture references an unknown pattern")
            }
            Self::FixtureGroupMismatch => {
                formatter.write_str("executable fixture and pattern groups differ")
            }
            Self::PartialExpectsAbsence => {
                formatter.write_str("partial fixture cannot expect a missing pattern")
            }
            Self::InvalidEvidenceExpectation => {
                formatter.write_str("fixture evidence expectation is invalid")
            }
            Self::InvalidRange => formatter.write_str("fixture expectation range is invalid"),
            Self::MalformedWithoutDiagnostic => {
                formatter.write_str("malformed fixture must expect a typed diagnostic")
            }
            Self::MissingGroupDefinition(group) => {
                write!(formatter, "pattern group {group} has no data definition")
            }
            Self::MissingFixtureClass { group, kind } => write!(
                formatter,
                "pattern group {group} is missing a {kind:?} executable fixture"
            ),
            Self::FixtureEscape => {
                formatter.write_str("fixture root escapes the pattern-pack directory")
            }
            Self::SymlinkEscape => {
                formatter.write_str("fixture symlink escapes the pattern-pack directory")
            }
            Self::FixtureDepthExceeded => {
                formatter.write_str("fixture tree exceeds its depth budget")
            }
            Self::FixtureEntriesExceeded => {
                formatter.write_str("fixture tree exceeds its entry budget")
            }
            Self::FixtureFileTooLarge => {
                formatter.write_str("fixture file exceeds its byte budget")
            }
            Self::FixtureTotalTooLarge => {
                formatter.write_str("fixture tree exceeds its total byte budget")
            }
            Self::Fingerprint => {
                formatter.write_str("failed to fingerprint executable pattern pack")
            }
        }
    }
}

impl std::error::Error for PatternPackLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPredicate(error) => Some(error),
            _ => None,
        }
    }
}
