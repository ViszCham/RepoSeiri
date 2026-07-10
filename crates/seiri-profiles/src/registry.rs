use crate::ProfileRuleDefinition;
use seiri_core::ProfileKind;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDefinition {
    pub profile: ProfileKind,
    pub rules: Vec<ProfileRuleDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRegistry {
    definitions: Vec<ProfileDefinition>,
}

impl ProfileRegistry {
    pub fn try_complete(definitions: Vec<ProfileDefinition>) -> Result<Self, ProfileRegistryError> {
        let registry = Self { definitions };
        registry.validate_complete()?;
        Ok(registry)
    }

    #[must_use]
    pub fn definitions(&self) -> &[ProfileDefinition] {
        &self.definitions
    }

    #[must_use]
    pub fn definition(&self, profile: ProfileKind) -> Option<&ProfileDefinition> {
        self.definitions
            .iter()
            .find(|definition| definition.profile == profile)
    }

    pub fn validate_complete(&self) -> Result<(), ProfileRegistryError> {
        let mut profiles = BTreeSet::new();
        for definition in &self.definitions {
            if !profiles.insert(definition.profile) {
                return Err(ProfileRegistryError::DuplicateProfile(definition.profile));
            }
            if definition.rules.is_empty() {
                return Err(ProfileRegistryError::EmptyProfile(definition.profile));
            }
            let mut pattern_ids = BTreeSet::new();
            for rule in &definition.rules {
                if !pattern_ids.insert(rule.pattern_id) {
                    return Err(ProfileRegistryError::DuplicateRule {
                        profile: definition.profile,
                        pattern_id: rule.pattern_id,
                    });
                }
            }
        }

        for profile in ProfileKind::ALL {
            if !profiles.contains(&profile) {
                return Err(ProfileRegistryError::MissingProfile(profile));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRegistryError {
    DuplicateProfile(ProfileKind),
    EmptyProfile(ProfileKind),
    DuplicateRule {
        profile: ProfileKind,
        pattern_id: &'static str,
    },
    MissingProfile(ProfileKind),
}

impl std::fmt::Display for ProfileRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateProfile(profile) => write!(formatter, "duplicate profile '{profile}'"),
            Self::EmptyProfile(profile) => write!(formatter, "profile '{profile}' has no rules"),
            Self::DuplicateRule {
                profile,
                pattern_id,
            } => write!(
                formatter,
                "profile '{profile}' has duplicate rule '{pattern_id}'"
            ),
            Self::MissingProfile(profile) => write!(formatter, "profile '{profile}' is missing"),
        }
    }
}

impl std::error::Error for ProfileRegistryError {}
