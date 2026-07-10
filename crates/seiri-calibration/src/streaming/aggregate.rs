mod output;

use super::{StreamingCalibrationLimits, StreamingLimitKind};
use crate::CalibrationError;
use seiri_core::{BenchmarkRepoRecord, ObservedPattern, ProfileKind, RouteKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct PatternSlot(u16);

impl PatternSlot {
    fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct CatalogPattern {
    id: &'static str,
    route: Option<RouteKind>,
}

#[derive(Debug)]
struct PatternCatalog {
    patterns: Vec<CatalogPattern>,
    slots: BTreeMap<&'static str, PatternSlot>,
}

impl PatternCatalog {
    fn common() -> Result<Self, CalibrationError> {
        let registry = seiri_patterns::common_registry();
        let mut patterns = registry
            .definitions()
            .iter()
            .map(|definition| CatalogPattern {
                id: definition.id,
                route: definition.route,
            })
            .collect::<Vec<_>>();
        patterns.sort_by_key(|pattern| pattern.id);
        if patterns.len() > usize::from(u16::MAX) {
            return Err(CalibrationError::PatternCatalogTooLarge {
                patterns: patterns.len(),
            });
        }
        let slots = patterns
            .iter()
            .enumerate()
            .map(|(index, pattern)| (pattern.id, PatternSlot(index as u16)))
            .collect();
        Ok(Self { patterns, slots })
    }

    fn slot(&self, pattern_id: &str) -> Option<PatternSlot> {
        self.slots.get(pattern_id).copied()
    }

    fn pattern(&self, slot: PatternSlot) -> CatalogPattern {
        self.patterns[slot.index()]
    }

    fn len(&self) -> usize {
        self.patterns.len()
    }
}

#[derive(Debug, Default)]
struct PatternCounter {
    repositories: usize,
    observations: u64,
    profile_repositories: BTreeMap<ProfileKind, usize>,
}

#[derive(Debug, Default)]
struct RouteCounter {
    repositories: usize,
    observations: u64,
}

#[derive(Debug, Default)]
struct PendingCounter {
    repositories: usize,
    observations: u64,
    example_locations: Vec<String>,
}

#[derive(Debug)]
pub(super) struct StreamingAccumulator {
    catalog: PatternCatalog,
    patterns: Vec<PatternCounter>,
    co_occurrences: Vec<usize>,
    routes: BTreeMap<RouteKind, RouteCounter>,
    profiles: BTreeMap<ProfileKind, usize>,
    pending: BTreeMap<String, PendingCounter>,
    metadata_sources: BTreeSet<String>,
    records_seen: usize,
    max_buffered_line_bytes: usize,
    max_patterns_per_record: usize,
}

impl StreamingAccumulator {
    pub(super) fn new() -> Result<Self, CalibrationError> {
        let catalog = PatternCatalog::common()?;
        let pattern_count = catalog.len();
        let co_occurrence_count = pattern_count.checked_mul(pattern_count).ok_or(
            CalibrationError::PatternCatalogTooLarge {
                patterns: pattern_count,
            },
        )?;
        Ok(Self {
            catalog,
            patterns: (0..pattern_count)
                .map(|_| PatternCounter::default())
                .collect(),
            co_occurrences: vec![0; co_occurrence_count],
            routes: BTreeMap::new(),
            profiles: BTreeMap::new(),
            pending: BTreeMap::new(),
            metadata_sources: BTreeSet::new(),
            records_seen: 0,
            max_buffered_line_bytes: 0,
            max_patterns_per_record: 0,
        })
    }

    pub(super) fn observe_line(&mut self, line_bytes: usize, patterns: usize) {
        self.max_buffered_line_bytes = self.max_buffered_line_bytes.max(line_bytes);
        self.max_patterns_per_record = self.max_patterns_per_record.max(patterns);
    }

    pub(super) fn push_record(
        &mut self,
        record: BenchmarkRepoRecord,
        line: usize,
        limits: StreamingCalibrationLimits,
    ) -> Result<(), CalibrationError> {
        let metadata_source = record.metadata_source.trim();
        if !metadata_source.is_empty() && !self.metadata_sources.contains(metadata_source) {
            if self.metadata_sources.len() >= limits.max_metadata_sources() {
                return Err(CalibrationError::StreamingLimitExceeded {
                    line,
                    resource: StreamingLimitKind::MetadataSources,
                    limit: limits.max_metadata_sources(),
                    actual: self.metadata_sources.len() + 1,
                });
            }
            self.metadata_sources.insert(metadata_source.to_string());
        }

        let mut record_patterns = Vec::<PatternSlot>::new();
        let mut record_routes = Vec::<RouteKind>::new();
        let mut record_pending = Vec::<String>::new();

        for observed in &record.observed_patterns {
            let slot = observed
                .pattern_id
                .as_deref()
                .and_then(|pattern_id| self.catalog.slot(pattern_id));
            let route = observed
                .route
                .or_else(|| slot.and_then(|slot| self.catalog.pattern(slot).route));
            if let Some(route) = route {
                let route_counter = self.routes.entry(route).or_default();
                checked_add_u64(
                    &mut route_counter.observations,
                    u64::from(observed.count.max(1)),
                    line,
                    "route_observations",
                )?;
                record_routes.push(route);
            }

            if let Some(slot) = slot {
                checked_add_u64(
                    &mut self.patterns[slot.index()].observations,
                    u64::from(observed.count.max(1)),
                    line,
                    "pattern_observations",
                )?;
                record_patterns.push(slot);
            } else {
                let key = pending_key(observed);
                if !self.pending.contains_key(&key)
                    && self.pending.len() >= limits.max_pending_patterns()
                {
                    return Err(CalibrationError::StreamingLimitExceeded {
                        line,
                        resource: StreamingLimitKind::PendingPatterns,
                        limit: limits.max_pending_patterns(),
                        actual: self.pending.len() + 1,
                    });
                }
                let pending = self.pending.entry(key.clone()).or_default();
                checked_add_u64(
                    &mut pending.observations,
                    u64::from(observed.count.max(1)),
                    line,
                    "pending_observations",
                )?;
                if let Some(location) = &observed.location {
                    if pending.example_locations.len() < 3 {
                        pending
                            .example_locations
                            .push(format!("{}:{location}", record.repo_id));
                    }
                }
                record_pending.push(key);
            }
        }

        record_patterns.sort_unstable();
        record_patterns.dedup();
        record_routes.sort_unstable();
        record_routes.dedup();
        record_pending.sort_unstable();
        record_pending.dedup();

        for slot in &record_patterns {
            let pattern = &mut self.patterns[slot.index()];
            checked_increment(&mut pattern.repositories, line, "pattern_repositories")?;
            if let Some(profile) = record.profile_hint {
                checked_increment(
                    pattern.profile_repositories.entry(profile).or_insert(0),
                    line,
                    "pattern_profile_repositories",
                )?;
            }
        }
        for route in record_routes {
            checked_increment(
                &mut self.routes.entry(route).or_default().repositories,
                line,
                "route_repositories",
            )?;
        }
        for key in record_pending {
            checked_increment(
                &mut self
                    .pending
                    .get_mut(&key)
                    .expect("pending key inserted")
                    .repositories,
                line,
                "pending_repositories",
            )?;
        }

        let pattern_count = self.catalog.len();
        for slot in &record_patterns {
            for other in &record_patterns {
                if slot == other {
                    continue;
                }
                let index = slot.index() * pattern_count + other.index();
                checked_increment(
                    &mut self.co_occurrences[index],
                    line,
                    "pattern_co_occurrences",
                )?;
            }
        }

        if let Some(profile) = record.profile_hint {
            checked_increment(
                self.profiles.entry(profile).or_insert(0),
                line,
                "profile_repositories",
            )?;
        }
        checked_increment(&mut self.records_seen, line, "records_seen")?;
        Ok(())
    }
}

fn pending_key(observed: &ObservedPattern) -> String {
    observed
        .pattern_id
        .as_ref()
        .filter(|pattern_id| !pattern_id.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| observed.raw_label.clone())
}

fn checked_increment(
    counter: &mut usize,
    line: usize,
    name: &'static str,
) -> Result<(), CalibrationError> {
    *counter = counter
        .checked_add(1)
        .ok_or(CalibrationError::CounterOverflow {
            line,
            counter: name,
        })?;
    Ok(())
}

fn checked_add_u64(
    counter: &mut u64,
    value: u64,
    line: usize,
    name: &'static str,
) -> Result<(), CalibrationError> {
    *counter = counter
        .checked_add(value)
        .ok_or(CalibrationError::CounterOverflow {
            line,
            counter: name,
        })?;
    Ok(())
}
