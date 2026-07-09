use seiri_core::{
    stable_id, BenchmarkDataset, BenchmarkRepoRecord, CalibrationConfidence,
    CalibrationReviewStatus, CalibrationRun, CalibrationSummary, EvidenceSchemaVersion,
    ObservedPattern, PatternCoOccurrence, PatternStats, PendingPatternCandidate, ProfileKind,
    ProfilePatternCorrelation, ProfilePriority, WeightSuggestion, SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
pub enum CalibrationError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Jsonl { line: usize, message: String },
}

impl Display for CalibrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Jsonl { line, message } => {
                write!(f, "invalid JSONL record at line {line}: {message}")
            }
        }
    }
}

impl std::error::Error for CalibrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Jsonl { .. } => None,
        }
    }
}

impl From<std::io::Error> for CalibrationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for CalibrationError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[must_use]
pub fn default_evidence_schema() -> EvidenceSchemaVersion {
    EvidenceSchemaVersion {
        schema_version: SCHEMA_VERSION.to_string(),
        compatible_from: "seiri.block_a.v1".to_string(),
        note: "Calibration input is evidence-derived and must not be treated as proof or automatic rule adoption.".to_string(),
    }
}

pub fn load_dataset(path: impl AsRef<Path>) -> Result<BenchmarkDataset, CalibrationError> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
    {
        let records = read_jsonl_records(reader)?;
        Ok(dataset_from_records(path, records))
    } else {
        Ok(serde_json::from_reader(reader)?)
    }
}

pub fn read_jsonl_records<R: BufRead>(
    reader: R,
) -> Result<Vec<BenchmarkRepoRecord>, CalibrationError> {
    let mut records = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record = serde_json::from_str::<BenchmarkRepoRecord>(trimmed).map_err(|error| {
            CalibrationError::Jsonl {
                line: index + 1,
                message: error.to_string(),
            }
        })?;
        records.push(record);
    }
    Ok(records)
}

#[must_use]
pub fn calibrate_dataset(dataset: &BenchmarkDataset) -> CalibrationRun {
    let known_patterns = known_pattern_ids();
    let current_weights = current_weight_map();
    let profile_totals = profile_totals(&dataset.records);
    let profiles = profiles_for_suggestions(&profile_totals);
    let mut stats = BTreeMap::<String, StatAccumulator>::new();
    let mut pending = BTreeMap::<String, PendingAccumulator>::new();

    for record in &dataset.records {
        let mut repo_patterns = BTreeSet::new();
        for observed in &record.observed_patterns {
            if let Some(pattern_id) = known_pattern_id(observed, &known_patterns) {
                let entry = stats.entry(pattern_id.clone()).or_default();
                entry.repositories.insert(record.repo_id.clone());
                entry.observations += u64::from(observed.count.max(1));
                if let Some(profile) = record.profile_hint {
                    entry
                        .profile_repositories
                        .entry(profile)
                        .or_default()
                        .insert(record.repo_id.clone());
                }
                repo_patterns.insert(pattern_id);
            } else {
                let key = pending_key(observed);
                let entry = pending
                    .entry(key.clone())
                    .or_insert_with(|| PendingAccumulator {
                        raw_label: key,
                        ..PendingAccumulator::default()
                    });
                entry.repositories.insert(record.repo_id.clone());
                entry.observations += u64::from(observed.count.max(1));
                if let Some(location) = &observed.location {
                    if entry.example_locations.len() < 3 {
                        entry
                            .example_locations
                            .push(format!("{}:{location}", record.repo_id));
                    }
                }
            }
        }
        add_co_occurrences(&mut stats, &record.repo_id, &repo_patterns);
    }

    let pattern_stats = build_pattern_stats(&stats, dataset.records.len(), &profile_totals);
    let pending_patterns = build_pending_patterns(pending);
    let weight_suggestions =
        build_weight_suggestions(&pattern_stats, &profiles, &current_weights, &profile_totals);

    CalibrationRun {
        schema_version: SCHEMA_VERSION.to_string(),
        run_id: stable_id("calibration-run", 1),
        dataset_id: dataset.dataset_id.clone(),
        summary: CalibrationSummary {
            records: dataset.records.len(),
            known_pattern_stats: pattern_stats.len(),
            pending_patterns: pending_patterns.len(),
            weight_suggestions: weight_suggestions.len(),
        },
        stats: pattern_stats,
        pending_patterns,
        weight_suggestions,
        claim_boundary: "Calibration output is candidate evidence for human review. RepoSeiri does not automatically adopt unverified rules or make truth, popularity, trust, security, or quality claims from this run.".to_string(),
    }
}

#[derive(Debug, Default)]
struct StatAccumulator {
    repositories: BTreeSet<String>,
    observations: u64,
    profile_repositories: BTreeMap<ProfileKind, BTreeSet<String>>,
    co_repositories: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Default)]
struct PendingAccumulator {
    raw_label: String,
    repositories: BTreeSet<String>,
    observations: u64,
    example_locations: Vec<String>,
}

fn dataset_from_records(path: &Path, records: Vec<BenchmarkRepoRecord>) -> BenchmarkDataset {
    let dataset_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("jsonl-dataset")
        .to_string();
    BenchmarkDataset {
        schema_version: SCHEMA_VERSION.to_string(),
        dataset_id: dataset_id.clone(),
        name: dataset_id,
        collected_at: "unknown".to_string(),
        extraction_conditions: vec!["JSONL records supplied without aggregate metadata.".to_string()],
        limitations: vec!["JSONL wrapper metadata is synthetic; review source provenance before adopting suggestions.".to_string()],
        evidence_schema: default_evidence_schema(),
        records,
    }
}

fn known_pattern_ids() -> BTreeSet<String> {
    seiri_patterns::common_registry()
        .definitions()
        .iter()
        .map(|definition| definition.id.to_string())
        .collect()
}

fn current_weight_map() -> BTreeMap<(ProfileKind, String), u32> {
    let mut map = BTreeMap::new();
    for profile in all_profiles() {
        for rule in seiri_profiles::profile_rules(profile) {
            map.insert((profile, rule.pattern_id.to_string()), rule.weight);
        }
    }
    map
}

fn profile_totals(records: &[BenchmarkRepoRecord]) -> BTreeMap<ProfileKind, usize> {
    let mut totals = BTreeMap::new();
    for record in records {
        if let Some(profile) = record.profile_hint {
            *totals.entry(profile).or_insert(0) += 1;
        }
    }
    totals
}

fn profiles_for_suggestions(profile_totals: &BTreeMap<ProfileKind, usize>) -> Vec<ProfileKind> {
    if profile_totals.is_empty() {
        vec![ProfileKind::Common]
    } else {
        profile_totals.keys().copied().collect()
    }
}

fn known_pattern_id(
    observed: &ObservedPattern,
    known_patterns: &BTreeSet<String>,
) -> Option<String> {
    observed
        .pattern_id
        .as_ref()
        .filter(|pattern_id| known_patterns.contains(*pattern_id))
        .cloned()
}

fn pending_key(observed: &ObservedPattern) -> String {
    observed
        .pattern_id
        .as_ref()
        .filter(|pattern_id| !pattern_id.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| observed.raw_label.clone())
}

fn add_co_occurrences(
    stats: &mut BTreeMap<String, StatAccumulator>,
    repo_id: &str,
    repo_patterns: &BTreeSet<String>,
) {
    for pattern_id in repo_patterns {
        for other_id in repo_patterns {
            if pattern_id == other_id {
                continue;
            }
            if let Some(stat) = stats.get_mut(pattern_id) {
                stat.co_repositories
                    .entry(other_id.clone())
                    .or_default()
                    .insert(repo_id.to_string());
            }
        }
    }
}

fn build_pattern_stats(
    stats: &BTreeMap<String, StatAccumulator>,
    total_records: usize,
    profile_totals: &BTreeMap<ProfileKind, usize>,
) -> Vec<PatternStats> {
    stats
        .iter()
        .map(|(pattern_id, stat)| {
            let repositories = stat.repositories.len();
            let profile_correlations = stat
                .profile_repositories
                .iter()
                .map(|(profile, repos)| ProfilePatternCorrelation {
                    profile: *profile,
                    repositories: repos.len(),
                    frequency_x1000: ratio_x1000(
                        repos.len(),
                        *profile_totals.get(profile).unwrap_or(&0),
                    ),
                })
                .collect();
            let co_occurrences = stat
                .co_repositories
                .iter()
                .map(|(other_id, repos)| PatternCoOccurrence {
                    pattern_id: other_id.clone(),
                    repositories: repos.len(),
                    co_frequency_x1000: ratio_x1000(repos.len(), repositories),
                })
                .collect();
            let confidence = confidence_for(total_records, repositories);
            PatternStats {
                pattern_id: pattern_id.clone(),
                repositories,
                observations: stat.observations,
                frequency_x1000: ratio_x1000(repositories, total_records),
                profile_correlations,
                co_occurrences,
                confidence,
                confidence_note: confidence_note(total_records, repositories, confidence),
            }
        })
        .collect()
}

fn build_pending_patterns(
    pending: BTreeMap<String, PendingAccumulator>,
) -> Vec<PendingPatternCandidate> {
    pending
        .into_iter()
        .enumerate()
        .map(|(index, (_, item))| PendingPatternCandidate {
            id: stable_id("pending-pattern", index + 1),
            raw_label: item.raw_label,
            observed_repositories: item.repositories.len(),
            observations: item.observations,
            example_locations: item.example_locations,
            review_status: CalibrationReviewStatus::PendingReview,
        })
        .collect()
}

fn build_weight_suggestions(
    stats: &[PatternStats],
    profiles: &[ProfileKind],
    current_weights: &BTreeMap<(ProfileKind, String), u32>,
    profile_totals: &BTreeMap<ProfileKind, usize>,
) -> Vec<WeightSuggestion> {
    let mut suggestions = Vec::new();
    for stat in stats {
        for profile in profiles {
            let profile_frequency = frequency_for_profile(stat, *profile, profile_totals)
                .unwrap_or(stat.frequency_x1000);
            let support = support_for_suggestion(stat, *profile, profile_totals);
            if support == 0 {
                continue;
            }
            let suggested_weight = weight_from_frequency(profile_frequency);
            let current_weight = current_weights
                .get(&(*profile, stat.pattern_id.clone()))
                .copied();
            if current_weight.is_some_and(|current| current == suggested_weight) {
                continue;
            }
            let suggested_delta = suggested_weight as i32 - current_weight.unwrap_or(0) as i32;
            suggestions.push(WeightSuggestion {
                id: stable_id("weight-suggestion", suggestions.len() + 1),
                profile: *profile,
                pattern_id: stat.pattern_id.clone(),
                current_weight,
                suggested_weight,
                suggested_delta,
                priority: priority_from_weight(suggested_weight),
                support_repositories: support,
                frequency_x1000: profile_frequency,
                confidence: stat.confidence,
                review_status: CalibrationReviewStatus::PendingReview,
                rationale: "Candidate weight only. A maintainer must review source quality, sampling bias, and product intent before adopting this rule.".to_string(),
            });
        }
    }
    suggestions
}

fn frequency_for_profile(
    stat: &PatternStats,
    profile: ProfileKind,
    profile_totals: &BTreeMap<ProfileKind, usize>,
) -> Option<u16> {
    if !profile_totals.contains_key(&profile) {
        return None;
    }
    stat.profile_correlations
        .iter()
        .find(|correlation| correlation.profile == profile)
        .map(|correlation| correlation.frequency_x1000)
        .or(Some(0))
}

fn support_for_suggestion(
    stat: &PatternStats,
    profile: ProfileKind,
    profile_totals: &BTreeMap<ProfileKind, usize>,
) -> usize {
    if !profile_totals.contains_key(&profile) {
        return stat.repositories;
    }
    stat.profile_correlations
        .iter()
        .find(|correlation| correlation.profile == profile)
        .map_or(0, |correlation| correlation.repositories)
}

fn ratio_x1000(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        return 0;
    }
    numerator
        .saturating_mul(1000)
        .checked_div(denominator)
        .unwrap_or(0)
        .min(1000) as u16
}

fn confidence_for(total_records: usize, repositories: usize) -> CalibrationConfidence {
    if total_records < 30 || repositories < 5 {
        CalibrationConfidence::Low
    } else if total_records >= 300 && repositories.saturating_mul(100) >= total_records * 40 {
        CalibrationConfidence::High
    } else {
        CalibrationConfidence::Medium
    }
}

fn confidence_note(
    total_records: usize,
    repositories: usize,
    confidence: CalibrationConfidence,
) -> String {
    match confidence {
        CalibrationConfidence::Low => format!(
            "Low confidence: sample has {total_records} records and {repositories} supporting repositories."
        ),
        CalibrationConfidence::Medium => format!(
            "Medium confidence: sample has {total_records} records and {repositories} supporting repositories; review sampling bias before adoption."
        ),
        CalibrationConfidence::High => format!(
            "High local support in this dataset: {repositories} of {total_records} repositories; still requires review before adoption."
        ),
    }
}

fn weight_from_frequency(frequency_x1000: u16) -> u32 {
    match frequency_x1000 {
        900..=1000 => 30,
        700..=899 => 24,
        500..=699 => 18,
        300..=499 => 12,
        100..=299 => 6,
        _ => 3,
    }
}

fn priority_from_weight(weight: u32) -> ProfilePriority {
    match weight {
        24.. => ProfilePriority::Critical,
        15..=23 => ProfilePriority::High,
        7..=14 => ProfilePriority::Normal,
        _ => ProfilePriority::Low,
    }
}

fn all_profiles() -> [ProfileKind; 8] {
    [
        ProfileKind::Common,
        ProfileKind::Library,
        ProfileKind::Cli,
        ProfileKind::Infra,
        ProfileKind::Docs,
        ProfileKind::Tutorial,
        ProfileKind::Research,
        ProfileKind::Template,
    ]
}
