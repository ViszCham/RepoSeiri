use super::error::PatternPackLoadError;
use super::model::{
    DataPatternDefinition, EvidenceExpectation, ExecutableFixtureSpec, ExecutablePatternPack,
    FixtureExpectation, FixtureScanBudget, RelativeFixturePath,
    EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION, MAX_DATA_PATTERN_DEFINITIONS, MAX_EXECUTABLE_FIXTURES,
    MAX_FIXTURE_DEPTH, MAX_FIXTURE_ENTRIES, MAX_FIXTURE_EXPECTATIONS,
};
use crate::{
    PatternAdoptionStage, PatternFixtureKind, PredicateAtom, PredicateInstruction, PredicateProgram,
};
use seiri_core::{ClaimBoundaryKind, PatternGroup, PatternOutcome};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MAX_PACK_BYTES: u64 = 2 * 1024 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_DEFINITION_BOUNDARIES: usize = 16;
const MAX_EXPECTED_EVIDENCE_IDS: usize = 64;
const FNV1A64_OFFSET: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PatternPackWire {
    schema_version: String,
    id: String,
    version: String,
    definitions: Vec<DataPatternDefinition>,
    fixtures: Vec<ExecutableFixtureSpec>,
}

#[derive(Deserialize)]
struct PredicatePreflight {
    atoms: Vec<PredicateAtom>,
    instructions: Vec<PredicateInstruction>,
}

pub fn load_executable_pattern_pack(
    path: impl AsRef<Path>,
) -> Result<ExecutablePatternPack, PatternPackLoadError> {
    let path = path.as_ref();
    let source_metadata = fs::symlink_metadata(path).map_err(redacted_io)?;
    if source_metadata.file_type().is_symlink() {
        return Err(PatternPackLoadError::SymlinkSource);
    }
    if source_metadata.len() > MAX_PACK_BYTES {
        return Err(PatternPackLoadError::SourceTooLarge);
    }
    let bytes = fs::read(path).map_err(redacted_io)?;
    if bytes.len() as u64 > MAX_PACK_BYTES {
        return Err(PatternPackLoadError::SourceTooLarge);
    }
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| PatternPackLoadError::Json {
            line: error.line(),
            column: error.column(),
        })?;
    if let Some(fixtures) = raw.get("fixtures").and_then(serde_json::Value::as_array) {
        for fixture in fixtures {
            if let Some(root) = fixture.get("root").and_then(serde_json::Value::as_str) {
                RelativeFixturePath::try_new(root.to_string())?;
            }
        }
    }
    if let Some(definitions) = raw.get("definitions").and_then(serde_json::Value::as_array) {
        for definition in definitions {
            let Some(predicate) = definition.get("predicate") else {
                continue;
            };
            let preflight: PredicatePreflight =
                serde_json::from_value(predicate.clone()).map_err(|error| {
                    PatternPackLoadError::Json {
                        line: error.line(),
                        column: error.column(),
                    }
                })?;
            PredicateProgram::try_new(preflight.atoms, preflight.instructions)
                .map_err(PatternPackLoadError::InvalidPredicate)?;
        }
    }
    let wire: PatternPackWire =
        serde_json::from_value(raw).map_err(|error| PatternPackLoadError::Json {
            line: error.line(),
            column: error.column(),
        })?;
    validate_wire(&wire)?;

    let source = fs::canonicalize(path).map_err(redacted_io)?;
    let base = source
        .parent()
        .ok_or(PatternPackLoadError::MissingPackDirectory)?
        .to_path_buf();
    let mut fixture_roots = BTreeMap::new();
    for fixture in &wire.fixtures {
        let candidate = base.join(fixture.root.as_str());
        let root = fs::canonicalize(candidate).map_err(redacted_io)?;
        if !root.starts_with(&base) || !root.is_dir() {
            return Err(PatternPackLoadError::FixtureEscape);
        }
        validate_fixture_tree(&base, &root, fixture.scan_budget)?;
        fixture_roots.insert(fixture.id.clone(), root);
    }

    let fingerprint = fingerprint_wire(&wire)?;
    Ok(ExecutablePatternPack {
        schema_version: wire.schema_version.into_boxed_str(),
        id: wire.id.into_boxed_str(),
        version: wire.version.into_boxed_str(),
        definitions: wire.definitions.into_boxed_slice(),
        fixtures: wire.fixtures.into_boxed_slice(),
        fixture_roots,
        fingerprint: fingerprint.into_boxed_str(),
    })
}

fn validate_wire(wire: &PatternPackWire) -> Result<(), PatternPackLoadError> {
    if wire.schema_version != EXECUTABLE_PATTERN_PACK_SCHEMA_VERSION {
        return Err(PatternPackLoadError::UnsupportedSchema);
    }
    validate_identifier(&wire.id)?;
    validate_identifier(&wire.version)?;
    if wire.definitions.is_empty() || wire.definitions.len() > MAX_DATA_PATTERN_DEFINITIONS {
        return Err(PatternPackLoadError::InvalidDefinitionCount);
    }
    if wire.fixtures.is_empty() || wire.fixtures.len() > MAX_EXECUTABLE_FIXTURES {
        return Err(PatternPackLoadError::InvalidFixtureCount);
    }

    let mut definition_ids = BTreeSet::new();
    for definition in &wire.definitions {
        validate_identifier(&definition.id)?;
        if !definition_ids.insert(definition.id.as_str()) {
            return Err(PatternPackLoadError::DuplicateDefinition);
        }
        definition
            .predicate
            .validate()
            .map_err(PatternPackLoadError::InvalidPredicate)?;
        if definition.adoption_stage != PatternAdoptionStage::Candidate {
            return Err(PatternPackLoadError::AutomaticAdoption);
        }
        if definition.boundaries.is_empty()
            || definition.boundaries.len() > MAX_DEFINITION_BOUNDARIES
            || definition.boundaries.iter().collect::<BTreeSet<_>>().len()
                != definition.boundaries.len()
        {
            return Err(PatternPackLoadError::InvalidBoundaries);
        }
        if !definition
            .boundaries
            .contains(&ClaimBoundaryKind::NotAutomaticPolicyAdoption)
            || !definition
                .boundaries
                .contains(&ClaimBoundaryKind::NotAutomaticWeightAdoption)
        {
            return Err(PatternPackLoadError::MissingAdoptionBoundary);
        }
    }

    let mut fixture_ids = BTreeSet::new();
    let mut coverage = BTreeSet::new();
    for fixture in &wire.fixtures {
        validate_identifier(&fixture.id)?;
        fixture.scan_budget.validate()?;
        if fixture.expectations.is_empty() || fixture.expectations.len() > MAX_FIXTURE_EXPECTATIONS
        {
            return Err(PatternPackLoadError::InvalidExpectationCount);
        }
        if !fixture_ids.insert(fixture.id.as_str()) {
            return Err(PatternPackLoadError::DuplicateFixture);
        }
        for expectation in &fixture.expectations {
            validate_expectation(fixture, expectation, &wire.definitions)?;
        }
        if fixture.kind == PatternFixtureKind::Malformed
            && !fixture
                .expectations
                .iter()
                .any(|expectation| matches!(expectation, FixtureExpectation::Diagnostic { .. }))
        {
            return Err(PatternPackLoadError::MalformedWithoutDiagnostic);
        }
        coverage.insert((fixture.group, fixture.kind));
    }
    for group in PatternGroup::ALL {
        if !wire
            .definitions
            .iter()
            .any(|definition| definition.group == group)
        {
            return Err(PatternPackLoadError::MissingGroupDefinition(group));
        }
        for kind in PatternFixtureKind::ALL {
            if !coverage.contains(&(group, kind)) {
                return Err(PatternPackLoadError::MissingFixtureClass { group, kind });
            }
        }
    }
    Ok(())
}

fn validate_expectation(
    fixture: &ExecutableFixtureSpec,
    expectation: &FixtureExpectation,
    definitions: &[DataPatternDefinition],
) -> Result<(), PatternPackLoadError> {
    match expectation {
        FixtureExpectation::Pattern {
            pattern,
            outcome,
            evidence,
        } => {
            let definition = definitions
                .iter()
                .find(|definition| definition.id == *pattern)
                .ok_or(PatternPackLoadError::UnknownPattern)?;
            if definition.group != fixture.group {
                return Err(PatternPackLoadError::FixtureGroupMismatch);
            }
            if fixture.kind == PatternFixtureKind::Partial && *outcome == PatternOutcome::Missing {
                return Err(PatternPackLoadError::PartialExpectsAbsence);
            }
            if let EvidenceExpectation::AtLeast(minimum) = evidence {
                if *minimum == 0 {
                    return Err(PatternPackLoadError::InvalidEvidenceExpectation);
                }
            }
            if let EvidenceExpectation::Exact(ids) = evidence {
                if ids.len() > MAX_EXPECTED_EVIDENCE_IDS
                    || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
                {
                    return Err(PatternPackLoadError::InvalidEvidenceExpectation);
                }
            }
        }
        FixtureExpectation::Gap {
            minimum, maximum, ..
        } if minimum > maximum => return Err(PatternPackLoadError::InvalidRange),
        FixtureExpectation::Diagnostic { minimum } if *minimum == 0 => {
            return Err(PatternPackLoadError::InvalidRange)
        }
        FixtureExpectation::Coverage { .. }
        | FixtureExpectation::Gap { .. }
        | FixtureExpectation::ClaimBoundary { .. }
        | FixtureExpectation::Diagnostic { .. } => {}
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), PatternPackLoadError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(PatternPackLoadError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_fixture_tree(
    base: &Path,
    root: &Path,
    budget: FixtureScanBudget,
) -> Result<(), PatternPackLoadError> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut entries = 0usize;
    let mut total_bytes = 0u64;
    while let Some((directory, depth)) = stack.pop() {
        if depth > MAX_FIXTURE_DEPTH {
            return Err(PatternPackLoadError::FixtureDepthExceeded);
        }
        let children = fs::read_dir(directory).map_err(redacted_io)?;
        for child in children {
            let child = child.map_err(redacted_io)?;
            entries += 1;
            if entries > MAX_FIXTURE_ENTRIES {
                return Err(PatternPackLoadError::FixtureEntriesExceeded);
            }
            let metadata = fs::symlink_metadata(child.path()).map_err(redacted_io)?;
            if metadata.file_type().is_symlink() {
                let target = fs::canonicalize(child.path()).map_err(redacted_io)?;
                if !target.starts_with(base) {
                    return Err(PatternPackLoadError::SymlinkEscape);
                }
                continue;
            }
            if metadata.is_dir() {
                stack.push((child.path(), depth + 1));
            } else if metadata.is_file() {
                if metadata.len() > budget.max_file_bytes {
                    return Err(PatternPackLoadError::FixtureFileTooLarge);
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .ok_or(PatternPackLoadError::FixtureTotalTooLarge)?;
                if total_bytes > budget.max_total_bytes {
                    return Err(PatternPackLoadError::FixtureTotalTooLarge);
                }
            }
        }
    }
    Ok(())
}

fn fingerprint_wire(wire: &PatternPackWire) -> Result<String, PatternPackLoadError> {
    let bytes = serde_json::to_vec(wire).map_err(|_| PatternPackLoadError::Fingerprint)?;
    let mut state = FNV1A64_OFFSET;
    for byte in bytes {
        state ^= u64::from(byte);
        state = state.wrapping_mul(FNV1A64_PRIME);
    }
    Ok(format!("fnv1a64:{state:016x}"))
}

fn redacted_io(error: std::io::Error) -> PatternPackLoadError {
    PatternPackLoadError::Io(error.kind())
}
