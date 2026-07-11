mod adoption;
mod error;
mod loader;
mod model;

pub use adoption::{
    evaluate_adoption_gate, AdoptionBlocker, AdoptionGateDecision, PatternAdoptionReview,
};
pub use error::PatternPackLoadError;
pub use loader::load_executable_pattern_pack;
pub use model::{
    DataPatternDefinition, EvidenceExpectation, ExecutableFixtureSpec, ExecutablePatternPack,
    FixtureExecutionResult, FixtureExecutionStatus, FixtureExpectation, FixtureExpectationActual,
    FixtureExpectationResult, FixtureScanBudget, FixtureSuiteReport, RelativeFixturePath,
    EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION, MAX_DATA_PATTERN_DEFINITIONS, MAX_EXECUTABLE_FIXTURES,
    MAX_FIXTURE_EXPECTATIONS,
};
