use crate::{common_registry, PatternRegistry, PatternRegistryError};
use seiri_core::{BenchmarkRepoRecord, CalibrationPatternPack, PatternGroup, ProfileKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

const MAX_PATTERN_PACK_FIXTURES: usize = 512;
const FNV1A64_OFFSET: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternFixtureKind {
    Positive,
    Negative,
    Ambiguous,
    Partial,
    Malformed,
}

impl PatternFixtureKind {
    pub const ALL: [Self; 5] = [
        Self::Positive,
        Self::Negative,
        Self::Ambiguous,
        Self::Partial,
        Self::Malformed,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Ambiguous => "ambiguous",
            Self::Partial => "partial",
            Self::Malformed => "malformed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternFixture {
    pub id: String,
    pub kind: PatternFixtureKind,
    pub group: PatternGroup,
    pub repository: String,
    pub pattern_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "profile", rename_all = "snake_case")]
pub enum PatternPackCondition {
    AllRecords,
    Profile(ProfileKind),
}

impl PatternPackCondition {
    #[must_use]
    pub fn matches(self, record: &BenchmarkRepoRecord) -> bool {
        match self {
            Self::AllRecords => true,
            Self::Profile(profile) => record.profile_hint == Some(profile),
        }
    }

    #[must_use]
    pub fn description(self) -> String {
        match self {
            Self::AllRecords => "all_records".to_string(),
            Self::Profile(profile) => format!("profile:{profile}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegistryFingerprint(u64);

impl Display for RegistryFingerprint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "fnv1a64:{:016x}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct PatternPack {
    id: String,
    version: String,
    condition: PatternPackCondition,
    registry: PatternRegistry,
    fixtures: Vec<PatternFixture>,
    fingerprint: RegistryFingerprint,
}

impl PatternPack {
    pub fn try_new(
        id: impl Into<String>,
        version: impl Into<String>,
        condition: PatternPackCondition,
        registry: PatternRegistry,
        mut fixtures: Vec<PatternFixture>,
    ) -> Result<Self, PatternPackError> {
        let id = id.into();
        let version = version.into();
        if id.trim().is_empty() || version.trim().is_empty() {
            return Err(PatternPackError::EmptyIdentity);
        }
        if fixtures.len() > MAX_PATTERN_PACK_FIXTURES {
            return Err(PatternPackError::TooManyFixtures(fixtures.len()));
        }
        registry
            .validate_complete()
            .map_err(PatternPackError::Registry)?;
        fixtures.sort_by(|left, right| left.id.cmp(&right.id));
        validate_fixtures(&registry, &fixtures)?;
        let fingerprint = fingerprint_for(&id, &version, condition, &registry, &fixtures);
        Ok(Self {
            id,
            version,
            condition,
            registry,
            fixtures,
            fingerprint,
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub const fn condition(&self) -> PatternPackCondition {
        self.condition
    }

    #[must_use]
    pub fn registry(&self) -> &PatternRegistry {
        &self.registry
    }

    #[must_use]
    pub fn fixtures(&self) -> &[PatternFixture] {
        &self.fixtures
    }

    #[must_use]
    pub const fn fingerprint(&self) -> RegistryFingerprint {
        self.fingerprint
    }

    #[must_use]
    pub fn matches_record(&self, record: &BenchmarkRepoRecord) -> bool {
        self.condition.matches(record)
    }

    #[must_use]
    pub fn calibration_metadata_for_counts(
        &self,
        eligible_records: usize,
        excluded_records: usize,
    ) -> CalibrationPatternPack {
        CalibrationPatternPack {
            id: self.id.clone(),
            version: self.version.clone(),
            condition: self.condition.description(),
            registry_fingerprint: self.fingerprint.to_string(),
            eligible_records,
            excluded_records,
        }
    }
}

#[must_use]
pub fn common_pattern_pack() -> PatternPack {
    pattern_pack("common", PatternPackCondition::AllRecords)
}

#[must_use]
pub fn profile_pattern_pack(profile: ProfileKind) -> PatternPack {
    pattern_pack(
        &format!("profile-{profile}"),
        PatternPackCondition::Profile(profile),
    )
}

fn pattern_pack(id: &str, condition: PatternPackCondition) -> PatternPack {
    let registry = common_registry();
    let fixtures = fixtures_for_registry(&registry);
    PatternPack::try_new(id, "pattern_pack.v4", condition, registry, fixtures)
        .expect("built-in pattern pack must satisfy fixture and registry invariants")
}

fn fixtures_for_registry(registry: &PatternRegistry) -> Vec<PatternFixture> {
    let mut fixtures = Vec::with_capacity(PatternGroup::ALL.len() * PatternFixtureKind::ALL.len());
    for group in PatternGroup::ALL {
        let definition = registry
            .definitions()
            .iter()
            .find(|definition| definition.group == group)
            .expect("complete registry provides one pattern per group");
        for kind in PatternFixtureKind::ALL {
            fixtures.push(PatternFixture {
                id: format!(
                    "fixture.{}.{}",
                    group.code().to_ascii_lowercase(),
                    kind.slug()
                ),
                kind,
                group,
                repository: format!("pack-{}-{}", group.code().to_ascii_lowercase(), kind.slug()),
                pattern_id: definition.id.to_string(),
            });
        }
    }
    fixtures
}

fn validate_fixtures(
    registry: &PatternRegistry,
    fixtures: &[PatternFixture],
) -> Result<(), PatternPackError> {
    let mut ids = BTreeSet::new();
    let mut coverage = BTreeSet::new();
    for fixture in fixtures {
        if fixture.id.trim().is_empty() || fixture.repository.trim().is_empty() {
            return Err(PatternPackError::EmptyFixture(fixture.id.clone()));
        }
        if !ids.insert(fixture.id.as_str()) {
            return Err(PatternPackError::DuplicateFixture(fixture.id.clone()));
        }
        let definition = registry.definition(&fixture.pattern_id).ok_or_else(|| {
            PatternPackError::UnknownFixturePattern {
                fixture_id: fixture.id.clone(),
                pattern_id: fixture.pattern_id.clone(),
            }
        })?;
        if definition.group != fixture.group {
            return Err(PatternPackError::FixtureGroupMismatch {
                fixture_id: fixture.id.clone(),
                fixture_group: fixture.group,
                pattern_group: definition.group,
            });
        }
        coverage.insert((fixture.group, fixture.kind));
    }
    for group in PatternGroup::ALL {
        for kind in PatternFixtureKind::ALL {
            if !coverage.contains(&(group, kind)) {
                return Err(PatternPackError::MissingFixture { group, kind });
            }
        }
    }
    Ok(())
}

fn fingerprint_for(
    id: &str,
    version: &str,
    condition: PatternPackCondition,
    registry: &PatternRegistry,
    fixtures: &[PatternFixture],
) -> RegistryFingerprint {
    let mut items = vec![
        format!("pack:{id}"),
        format!("version:{version}"),
        format!("condition:{}", condition.description()),
    ];
    items.extend(registry.definitions().iter().map(|definition| {
        format!(
            "pattern:{}:{:?}:{}:{:?}:{}:{}:{}",
            definition.id,
            definition.group,
            definition.title,
            definition.route,
            definition.detector.basis(),
            definition.detector.label(),
            definition.adoption_stage.as_str(),
        )
    }));
    items.extend(fixtures.iter().map(|fixture| {
        format!(
            "fixture:{}:{:?}:{:?}:{}:{}",
            fixture.id, fixture.kind, fixture.group, fixture.repository, fixture.pattern_id
        )
    }));
    items.sort();

    let mut state = FNV1A64_OFFSET;
    for item in items {
        for byte in item.bytes().chain([0]) {
            state ^= u64::from(byte);
            state = state.wrapping_mul(FNV1A64_PRIME);
        }
    }
    RegistryFingerprint(state)
}

#[derive(Debug)]
pub enum PatternPackError {
    EmptyIdentity,
    TooManyFixtures(usize),
    Registry(PatternRegistryError),
    EmptyFixture(String),
    DuplicateFixture(String),
    UnknownFixturePattern {
        fixture_id: String,
        pattern_id: String,
    },
    FixtureGroupMismatch {
        fixture_id: String,
        fixture_group: PatternGroup,
        pattern_group: PatternGroup,
    },
    MissingFixture {
        group: PatternGroup,
        kind: PatternFixtureKind,
    },
}

impl Display for PatternPackError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentity => formatter.write_str("pattern pack id and version must not be empty"),
            Self::TooManyFixtures(count) => write!(
                formatter,
                "pattern pack has {count} fixtures; maximum is {MAX_PATTERN_PACK_FIXTURES}"
            ),
            Self::Registry(error) => write!(formatter, "pattern pack registry is invalid: {error}"),
            Self::EmptyFixture(id) => write!(formatter, "pattern fixture '{id}' has an empty field"),
            Self::DuplicateFixture(id) => write!(formatter, "duplicate pattern fixture '{id}'"),
            Self::UnknownFixturePattern {
                fixture_id,
                pattern_id,
            } => write!(
                formatter,
                "pattern fixture '{fixture_id}' references unknown pattern '{pattern_id}'"
            ),
            Self::FixtureGroupMismatch {
                fixture_id,
                fixture_group,
                pattern_group,
            } => write!(
                formatter,
                "pattern fixture '{fixture_id}' group {fixture_group} does not match pattern group {pattern_group}"
            ),
            Self::MissingFixture { group, kind } => write!(
                formatter,
                "pattern pack is missing {:?} fixture coverage for group {group}",
                kind
            ),
        }
    }
}

impl std::error::Error for PatternPackError {}
