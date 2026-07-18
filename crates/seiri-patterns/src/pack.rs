use crate::{common_registry, PatternRegistry, PatternRegistryError};
use seiri_core::{BenchmarkRepoRecord, CalibrationPatternPack, PatternGroup, ProfileKind};
use seiri_digest::{Digest32, StableHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

const COMMON_PATTERN_PACK_VERSION: &str = "seiri.pattern-pack.v1";
const REGISTRY_FINGERPRINT_DOMAIN: &[u8] = b"seiri.pattern-registry.semantic.v2";

const MAX_PATTERN_PACK_FIXTURES: usize = 512;

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
pub struct RegistryFingerprint(Digest32);

impl Display for RegistryFingerprint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
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
    PatternPack::try_new(
        id,
        COMMON_PATTERN_PACK_VERSION,
        condition,
        registry,
        fixtures,
    )
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
    #[derive(Serialize)]
    struct DefinitionWire<'a> {
        id: &'a str,
        group: &'a str,
        title: &'a str,
        route: Option<&'a str>,
        detector: &'a crate::PatternDetector,
        adoption_stage: &'a str,
        boundary: &'a crate::PatternBoundary,
        predicate: Option<&'a crate::PredicateProgram>,
    }

    let mut definitions = registry
        .definitions()
        .iter()
        .map(|definition| {
            serde_json::to_vec(&DefinitionWire {
                id: definition.id,
                group: definition.group.code(),
                title: definition.title,
                route: definition.route.map(route_tag),
                detector: &definition.detector,
                adoption_stage: definition.adoption_stage.as_str(),
                boundary: &definition.boundary,
                predicate: definition.predicate.as_ref(),
            })
            .expect("pattern fingerprint wire contains serializable values")
        })
        .collect::<Vec<_>>();
    definitions.sort();
    let mut fixture_wires = fixtures
        .iter()
        .map(|fixture| serde_json::to_vec(fixture).expect("fixture fingerprint wire"))
        .collect::<Vec<_>>();
    fixture_wires.sort();

    let mut hasher = StableHasher::new(REGISTRY_FINGERPRINT_DOMAIN, 7);
    hasher
        .str(1, id)
        .str(2, version)
        .str(3, &condition.description())
        .usize(4, definitions.len());
    for definition in definitions {
        hasher.field(5, &definition);
    }
    hasher.usize(6, fixture_wires.len());
    for fixture in fixture_wires {
        hasher.field(7, &fixture);
    }
    RegistryFingerprint(hasher.finish())
}

const fn route_tag(route: seiri_core::RouteKind) -> &'static str {
    match route {
        seiri_core::RouteKind::Identity => "identity",
        seiri_core::RouteKind::Docs => "docs",
        seiri_core::RouteKind::Quickstart => "quickstart",
        seiri_core::RouteKind::Support => "support",
        seiri_core::RouteKind::Intake => "intake",
        seiri_core::RouteKind::Contributing => "contributing",
        seiri_core::RouteKind::Security => "security",
        seiri_core::RouteKind::Release => "release",
        seiri_core::RouteKind::Lifecycle => "lifecycle",
        seiri_core::RouteKind::Governance => "governance",
        seiri_core::RouteKind::License => "license",
        seiri_core::RouteKind::Automation => "automation",
        seiri_core::RouteKind::Ownership => "ownership",
        seiri_core::RouteKind::Hygiene => "hygiene",
        seiri_core::RouteKind::Unknown => "unknown",
    }
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
