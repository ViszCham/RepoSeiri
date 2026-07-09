use seiri_core::{
    stable_id, BaselineRequirement, BenchmarkDataset, BenchmarkRepoRecord, CalibrationConfidence,
    CalibrationReviewStatus, CalibrationRun, CalibrationScale, CalibrationSource,
    CalibrationSourceKind, CalibrationSummary, ClaimBoundary, EvidenceSchemaVersion,
    ObservedPattern, PatternCoOccurrence, PatternStats, PendingPatternCandidate, ProfileBranch,
    ProfileKind, ProfilePatternCorrelation, ProfilePriority, RouteKind, RouteRequirement,
    WeightSuggestion, SCHEMA_VERSION,
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
    let pattern_routes = pattern_route_map();
    let current_weights = current_weight_map();
    let profile_totals = profile_totals(&dataset.records);
    let profiles = profiles_for_suggestions(&profile_totals);
    let sources = calibration_sources(dataset);
    let source_ids = source_ids(&sources);
    let mut stats = BTreeMap::<String, StatAccumulator>::new();
    let mut routes = BTreeMap::<RouteKind, RouteAccumulator>::new();
    let mut pending = BTreeMap::<String, PendingAccumulator>::new();

    for record in &dataset.records {
        let mut repo_patterns = BTreeSet::new();
        for observed in &record.observed_patterns {
            if let Some(route) = route_for_observed(observed, &pattern_routes) {
                let entry = routes.entry(route).or_default();
                entry.repositories.insert(record.repo_id.clone());
                entry.observations += u64::from(observed.count.max(1));
            }
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

    let pattern_stats = build_pattern_stats(
        &stats,
        dataset.records.len(),
        &profile_totals,
        &pattern_routes,
        &source_ids,
    );
    let route_requirements = build_route_requirements(&routes, dataset.records.len(), &source_ids);
    let profile_branches = build_profile_branches(&profile_totals, dataset.records.len());
    let pending_patterns = build_pending_patterns(pending, &source_ids);
    let weight_suggestions = build_weight_suggestions(
        &pattern_stats,
        &profiles,
        &current_weights,
        &profile_totals,
        &source_ids,
    );

    CalibrationRun {
        schema_version: SCHEMA_VERSION.to_string(),
        run_id: stable_id("calibration-run", 1),
        dataset_id: dataset.dataset_id.clone(),
        sources,
        summary: CalibrationSummary {
            records: dataset.records.len(),
            sources: source_ids.len(),
            known_pattern_stats: pattern_stats.len(),
            route_requirements: route_requirements.len(),
            profile_branches: profile_branches.len(),
            pending_patterns: pending_patterns.len(),
            weight_suggestions: weight_suggestions.len(),
        },
        stats: pattern_stats,
        route_requirements,
        profile_branches,
        pending_patterns,
        weight_suggestions,
        claim_boundary: default_claim_boundary(),
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
struct RouteAccumulator {
    repositories: BTreeSet<String>,
    observations: u64,
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
        name: dataset_id.clone(),
        collected_at: "unknown".to_string(),
        calibration_sources: vec![CalibrationSource {
            id: stable_id("calibration-source", 1),
            kind: CalibrationSourceKind::JsonlRecords,
            label: dataset_id.clone(),
            collected_at: "unknown".to_string(),
            records: records.len(),
            scale: scale_for_records(records.len()),
            metadata_sources: metadata_sources(&records),
            extraction_conditions: vec![
                "JSONL records supplied without aggregate metadata.".to_string(),
            ],
            limitations: vec![
                "JSONL wrapper metadata is synthetic; review source provenance before adopting suggestions."
                    .to_string(),
            ],
            evidence_schema: Some(default_evidence_schema()),
            review_status: CalibrationReviewStatus::PendingReview,
        }],
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

fn pattern_route_map() -> BTreeMap<String, RouteKind> {
    seiri_patterns::common_registry()
        .definitions()
        .iter()
        .filter_map(|definition| {
            definition
                .route
                .map(|route| (definition.id.to_string(), route))
        })
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

fn calibration_sources(dataset: &BenchmarkDataset) -> Vec<CalibrationSource> {
    if !dataset.calibration_sources.is_empty() {
        return dataset.calibration_sources.clone();
    }
    vec![CalibrationSource {
        id: stable_id("calibration-source", 1),
        kind: inferred_source_kind(dataset),
        label: dataset.name.clone(),
        collected_at: dataset.collected_at.clone(),
        records: dataset.records.len(),
        scale: scale_for_records(dataset.records.len()),
        metadata_sources: metadata_sources(&dataset.records),
        extraction_conditions: dataset.extraction_conditions.clone(),
        limitations: dataset.limitations.clone(),
        evidence_schema: Some(dataset.evidence_schema.clone()),
        review_status: CalibrationReviewStatus::PendingReview,
    }]
}

fn inferred_source_kind(dataset: &BenchmarkDataset) -> CalibrationSourceKind {
    if dataset.dataset_id.contains("fixture") || dataset.name.contains("Fixture") {
        CalibrationSourceKind::Fixture
    } else {
        CalibrationSourceKind::BenchmarkDataset
    }
}

fn source_ids(sources: &[CalibrationSource]) -> Vec<String> {
    sources.iter().map(|source| source.id.clone()).collect()
}

fn metadata_sources(records: &[BenchmarkRepoRecord]) -> Vec<String> {
    records
        .iter()
        .filter_map(|record| {
            let value = record.metadata_source.trim();
            (!value.is_empty()).then(|| value.to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn scale_for_records(records: usize) -> CalibrationScale {
    if records >= 1_000_000 {
        CalibrationScale::Million
    } else if records >= 100_000 {
        CalibrationScale::HundredK
    } else if records >= 1_000 {
        CalibrationScale::Small
    } else {
        CalibrationScale::Tiny
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

fn route_for_observed(
    observed: &ObservedPattern,
    pattern_routes: &BTreeMap<String, RouteKind>,
) -> Option<RouteKind> {
    observed.route.or_else(|| {
        observed
            .pattern_id
            .as_ref()
            .and_then(|pattern_id| pattern_routes.get(pattern_id).copied())
    })
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
    pattern_routes: &BTreeMap<String, RouteKind>,
    source_ids: &[String],
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
                route: pattern_routes.get(pattern_id).copied(),
                repositories,
                observations: stat.observations,
                frequency_x1000: ratio_x1000(repositories, total_records),
                source_ids: source_ids.to_vec(),
                profile_correlations,
                co_occurrences,
                confidence,
                confidence_note: confidence_note(total_records, repositories, confidence),
                review_status: CalibrationReviewStatus::PendingReview,
            }
        })
        .collect()
}

fn build_pending_patterns(
    pending: BTreeMap<String, PendingAccumulator>,
    source_ids: &[String],
) -> Vec<PendingPatternCandidate> {
    pending
        .into_iter()
        .enumerate()
        .map(|(index, (_, item))| PendingPatternCandidate {
            id: stable_id("pending-pattern", index + 1),
            raw_label: item.raw_label,
            observed_repositories: item.repositories.len(),
            observations: item.observations,
            source_ids: source_ids.to_vec(),
            example_locations: item.example_locations,
            review_status: CalibrationReviewStatus::PendingReview,
        })
        .collect()
}

fn build_route_requirements(
    routes: &BTreeMap<RouteKind, RouteAccumulator>,
    total_records: usize,
    source_ids: &[String],
) -> Vec<RouteRequirement> {
    routes
        .iter()
        .enumerate()
        .map(|(index, (route, item))| {
            let repositories = item.repositories.len();
            let frequency_x1000 = ratio_x1000(repositories, total_records);
            let suggested_requirement = route_requirement_from_frequency(frequency_x1000);
            let priority = priority_from_route_frequency(frequency_x1000);
            let confidence = confidence_for(total_records, repositories);
            RouteRequirement {
                id: stable_id("route-requirement", index + 1),
                route: *route,
                supporting_repositories: repositories,
                observations: item.observations,
                frequency_x1000,
                suggested_requirement,
                priority,
                source_ids: source_ids.to_vec(),
                confidence,
                review_status: CalibrationReviewStatus::PendingReview,
                rationale: format!(
                    "Reviewable route requirement candidate only. Route `{:?}` appeared in {repositories} of {total_records} records; maintainers must review source quality and repository purpose before adopting.",
                    route
                ),
            }
        })
        .collect()
}

fn build_profile_branches(
    profile_totals: &BTreeMap<ProfileKind, usize>,
    total_records: usize,
) -> Vec<ProfileBranch> {
    let mut branches = profile_totals
        .iter()
        .map(|(profile, repositories)| {
            let prior_x1000 = ratio_x1000(*repositories, total_records);
            let confidence_x100 = confidence_x100_for(total_records, *repositories);
            ProfileBranch {
                rank: 0,
                profile: *profile,
                prior_x1000,
                confidence_x100,
                evidence_score_x100: (prior_x1000 / 10).min(100) as u8,
                score_x100: confidence_x100,
                matched_signals: vec![
                    format!("profile_hint:{profile}"),
                    format!("records:{repositories}"),
                ],
                missing_signals: vec!["manual profile review".to_string()],
                rationale: "Dataset-level profile branch candidate only. It records observed profile hints and does not force runtime profile selection.".to_string(),
            }
        })
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        right
            .score_x100
            .cmp(&left.score_x100)
            .then_with(|| right.prior_x1000.cmp(&left.prior_x1000))
            .then_with(|| left.profile.cmp(&right.profile))
    });
    for (index, branch) in branches.iter_mut().enumerate() {
        branch.rank = index + 1;
    }
    branches
}

fn build_weight_suggestions(
    stats: &[PatternStats],
    profiles: &[ProfileKind],
    current_weights: &BTreeMap<(ProfileKind, String), u32>,
    profile_totals: &BTreeMap<ProfileKind, usize>,
    source_ids: &[String],
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
                route: stat.route,
                current_weight,
                suggested_weight,
                suggested_delta,
                priority: priority_from_weight(suggested_weight),
                support_repositories: support,
                frequency_x1000: profile_frequency,
                source_ids: source_ids.to_vec(),
                confidence: stat.confidence,
                review_status: CalibrationReviewStatus::PendingReview,
                rationale: "Reviewable calibration suggestion only. A maintainer must review source quality, sampling bias, route fit, and product intent before adopting this rule.".to_string(),
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

fn confidence_x100_for(total_records: usize, repositories: usize) -> u8 {
    if total_records == 0 {
        return 0;
    }
    let frequency = ratio_x1000(repositories, total_records) / 10;
    let sample_factor = if total_records >= 1_000_000 {
        100
    } else if total_records >= 100_000 {
        90
    } else if total_records >= 1_000 {
        75
    } else if total_records >= 30 {
        55
    } else {
        30
    };
    frequency.min(sample_factor) as u8
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

fn route_requirement_from_frequency(frequency_x1000: u16) -> BaselineRequirement {
    if frequency_x1000 >= 500 {
        BaselineRequirement::Required
    } else {
        BaselineRequirement::Optional
    }
}

fn priority_from_route_frequency(frequency_x1000: u16) -> ProfilePriority {
    match frequency_x1000 {
        800..=1000 => ProfilePriority::Critical,
        500..=799 => ProfilePriority::High,
        250..=499 => ProfilePriority::Normal,
        _ => ProfilePriority::Low,
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

fn all_profiles() -> [ProfileKind; 11] {
    [
        ProfileKind::Common,
        ProfileKind::Library,
        ProfileKind::Cli,
        ProfileKind::Infra,
        ProfileKind::Product,
        ProfileKind::Runtime,
        ProfileKind::Docs,
        ProfileKind::Tutorial,
        ProfileKind::Ml,
        ProfileKind::Research,
        ProfileKind::Template,
    ]
}

fn default_claim_boundary() -> ClaimBoundary {
    ClaimBoundary {
        summary: "Calibration output is candidate evidence for human review. RepoSeiri does not automatically adopt unverified rules or make truth, popularity, trust, security, or quality claims from this run.".to_string(),
        review_required: true,
        runtime_rule_adoption_allowed: false,
        automatic_weight_adoption_allowed: false,
        guarantee_allowed: false,
        blocked_claims: vec![
            "truth".to_string(),
            "popularity guarantee".to_string(),
            "trust guarantee".to_string(),
            "security guarantee".to_string(),
            "quality guarantee".to_string(),
            "automatic runtime rule adoption".to_string(),
        ],
    }
}
