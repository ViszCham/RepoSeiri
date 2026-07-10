use crate::{
    PatternBoundary, PatternDetector, PatternNegativeFixture, PredicateContext, PredicateProgram,
    PredicateProgramError,
};
use seiri_core::{stable_id, PatternGroup, PatternMatch, PatternOutcome, RepoSnapshot, RouteKind};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternDefinition {
    pub id: &'static str,
    pub group: PatternGroup,
    pub title: &'static str,
    pub route: Option<RouteKind>,
    pub detector: PatternDetector,
    pub adoption_stage: PatternAdoptionStage,
    pub boundary: PatternBoundary,
    pub predicate: Option<PredicateProgram>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternAdoptionStage {
    CommonBaseline,
    Candidate,
}

impl PatternAdoptionStage {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CommonBaseline => "common_baseline",
            Self::Candidate => "candidate",
        }
    }

    #[must_use]
    pub fn active_in_common_baseline(self) -> bool {
        matches!(self, Self::CommonBaseline)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternRegistry {
    definitions: Vec<PatternDefinition>,
    negative_fixtures: Vec<PatternNegativeFixture>,
}

impl PatternRegistry {
    #[must_use]
    pub fn new(definitions: Vec<PatternDefinition>) -> Self {
        Self {
            definitions,
            negative_fixtures: Vec::new(),
        }
    }

    pub fn try_complete(
        definitions: Vec<PatternDefinition>,
        negative_fixtures: Vec<PatternNegativeFixture>,
    ) -> Result<Self, PatternRegistryError> {
        let registry = Self {
            definitions,
            negative_fixtures,
        };
        registry.validate_complete()?;
        Ok(registry)
    }

    #[must_use]
    pub fn definitions(&self) -> &[PatternDefinition] {
        &self.definitions
    }

    #[must_use]
    pub fn negative_fixtures(&self) -> &[PatternNegativeFixture] {
        &self.negative_fixtures
    }

    pub fn negative_fixtures_for(
        &self,
        group: PatternGroup,
    ) -> impl Iterator<Item = &PatternNegativeFixture> {
        self.negative_fixtures
            .iter()
            .filter(move |fixture| fixture.group == group)
    }

    #[must_use]
    pub fn definition(&self, pattern_id: &str) -> Option<&PatternDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.id == pattern_id)
    }

    #[must_use]
    pub fn evaluation_definitions(&self) -> Vec<&PatternDefinition> {
        self.definitions
            .iter()
            .filter(|definition| definition.adoption_stage.active_in_common_baseline())
            .collect()
    }

    #[must_use]
    pub fn evaluate_patterns(&self, snapshot: &RepoSnapshot) -> Vec<PatternMatch> {
        self.evaluation_definitions()
            .into_iter()
            .enumerate()
            .map(|(index, definition)| {
                let evidence_ids = definition.detector.evidence_ids(snapshot);
                PatternMatch {
                    id: stable_id("pattern-match", index + 1),
                    pattern_id: definition.id.to_string(),
                    title: definition.title.to_string(),
                    route: definition.route,
                    outcome: if evidence_ids.is_empty() {
                        PatternOutcome::Missing
                    } else {
                        PatternOutcome::Present
                    },
                    evidence_ids,
                    basis: definition.detector.basis().to_string(),
                }
            })
            .collect()
    }

    #[must_use]
    pub fn evaluate_predicate(
        &self,
        pattern_id: &str,
        snapshot: &RepoSnapshot,
    ) -> Option<seiri_core::Observation<()>> {
        self.definition(pattern_id)
            .and_then(|definition| definition.predicate.as_ref())
            .map(|program| program.evaluate(PredicateContext::from_snapshot(snapshot)))
    }

    pub fn validate_complete(&self) -> Result<(), PatternRegistryError> {
        let mut pattern_ids = BTreeSet::new();
        for definition in &self.definitions {
            if !pattern_ids.insert(definition.id) {
                return Err(PatternRegistryError::DuplicatePatternId(definition.id));
            }
            if let Some(predicate) = &definition.predicate {
                predicate
                    .validate()
                    .map_err(|source| PatternRegistryError::InvalidPredicate {
                        pattern_id: definition.id,
                        source,
                    })?;
            }
        }

        let mut fixture_ids = BTreeSet::new();
        for fixture in &self.negative_fixtures {
            if !fixture_ids.insert(fixture.id) {
                return Err(PatternRegistryError::DuplicateFixtureId(fixture.id));
            }
            let definition = self.definition(fixture.pattern_id).ok_or(
                PatternRegistryError::UnknownFixturePattern {
                    fixture_id: fixture.id,
                    pattern_id: fixture.pattern_id,
                },
            )?;
            if definition.group != fixture.group {
                return Err(PatternRegistryError::FixtureGroupMismatch {
                    fixture_id: fixture.id,
                    fixture_group: fixture.group,
                    pattern_group: definition.group,
                });
            }
        }

        for group in PatternGroup::ALL {
            if !self
                .definitions
                .iter()
                .any(|definition| definition.group == group)
            {
                return Err(PatternRegistryError::MissingDetector(group));
            }
            if !self
                .negative_fixtures
                .iter()
                .any(|fixture| fixture.group == group)
            {
                return Err(PatternRegistryError::MissingNegativeFixture(group));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternRegistryError {
    DuplicatePatternId(&'static str),
    DuplicateFixtureId(&'static str),
    UnknownFixturePattern {
        fixture_id: &'static str,
        pattern_id: &'static str,
    },
    FixtureGroupMismatch {
        fixture_id: &'static str,
        fixture_group: PatternGroup,
        pattern_group: PatternGroup,
    },
    MissingDetector(PatternGroup),
    MissingNegativeFixture(PatternGroup),
    InvalidPredicate {
        pattern_id: &'static str,
        source: PredicateProgramError,
    },
}

impl std::fmt::Display for PatternRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePatternId(id) => write!(formatter, "duplicate pattern id '{id}'"),
            Self::DuplicateFixtureId(id) => write!(formatter, "duplicate fixture id '{id}'"),
            Self::UnknownFixturePattern {
                fixture_id,
                pattern_id,
            } => write!(
                formatter,
                "negative fixture '{fixture_id}' references unknown pattern '{pattern_id}'"
            ),
            Self::FixtureGroupMismatch {
                fixture_id,
                fixture_group,
                pattern_group,
            } => write!(
                formatter,
                "negative fixture '{fixture_id}' group {fixture_group} does not match pattern group {pattern_group}"
            ),
            Self::MissingDetector(group) => {
                write!(formatter, "pattern group {group} has no detector")
            }
            Self::MissingNegativeFixture(group) => {
                write!(formatter, "pattern group {group} has no negative fixture")
            }
            Self::InvalidPredicate { pattern_id, source } => {
                write!(formatter, "pattern '{pattern_id}' has an invalid predicate: {source}")
            }
        }
    }
}

impl std::error::Error for PatternRegistryError {}
