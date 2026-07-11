use super::{PatternSlot, StreamingAccumulator};
use crate::{
    build_profile_branches, build_weight_suggestions, confidence_for, confidence_note,
    current_weight_map, default_claim_boundary, default_evidence_schema,
    priority_from_route_frequency, profiles_for_suggestions, ratio_x1000,
    route_requirement_from_frequency, scale_for_records,
};
use seiri_core::{
    stable_id, CalibrationAggregationMode, CalibrationRecordIdentity, CalibrationReplayDigest,
    CalibrationResourceTrace, CalibrationReviewStatus, CalibrationRun, CalibrationSource,
    CalibrationSourceKind, CalibrationSourceVisibility, CalibrationSummary, PatternCoOccurrence,
    PatternStats, PendingPatternCandidate, ProfilePatternCorrelation, RouteRequirement,
    CALIBRATION_SCHEMA_VERSION,
};

impl StreamingAccumulator {
    pub(in crate::streaming) fn finish(
        self,
        metadata: crate::StreamingCalibrationMetadata,
        replay_digest: CalibrationReplayDigest,
    ) -> CalibrationRun {
        let pattern_pack = seiri_patterns::common_pattern_pack()
            .calibration_metadata_for_counts(self.records_seen, 0);
        let sources = vec![CalibrationSource {
            id: stable_id("calibration-source", 1),
            kind: CalibrationSourceKind::JsonlRecords,
            visibility: CalibrationSourceVisibility::LocalOnly,
            label: metadata.name,
            collected_at: metadata.collected_at,
            records: self.records_seen,
            scale: scale_for_records(self.records_seen),
            metadata_sources: self.metadata_sources.iter().cloned().collect(),
            extraction_conditions: vec![
                "JSONL records supplied without aggregate metadata.".to_string(),
                "Streaming mode treats each non-empty JSONL line as one repository record."
                    .to_string(),
            ],
            limitations: vec![
                "JSONL wrapper metadata is synthetic; review source provenance before adopting suggestions."
                    .to_string(),
                "Global repository-id uniqueness is an input-preparation responsibility; streaming mode retains no repository ids."
                    .to_string(),
            ],
            evidence_schema: Some(default_evidence_schema()),
            review_status: CalibrationReviewStatus::PendingReview,
        }];
        let source_ids = sources
            .iter()
            .map(|source| source.id.clone())
            .collect::<Vec<_>>();
        let pattern_stats = self.build_pattern_stats(&source_ids);
        let route_requirements = self.build_route_requirements(&source_ids);
        let profile_branches = build_profile_branches(&self.profiles, self.records_seen);
        let pending_patterns = self.build_pending_patterns(&source_ids);
        let profiles = profiles_for_suggestions(&self.profiles);
        let weight_suggestions = build_weight_suggestions(
            &pattern_stats,
            &profiles,
            &current_weight_map(),
            &self.profiles,
            &source_ids,
        );
        let profile_slots = self.profiles.len()
            + self
                .patterns
                .iter()
                .map(|pattern| pattern.profile_repositories.len())
                .sum::<usize>();
        let resource_trace = CalibrationResourceTrace {
            aggregation_mode: CalibrationAggregationMode::StreamingJsonl,
            record_identity: CalibrationRecordIdentity::OneNonemptyJsonlLinePerRepository,
            records_seen: self.records_seen,
            max_buffered_line_bytes: self.max_buffered_line_bytes,
            max_patterns_per_record: self.max_patterns_per_record,
            known_pattern_slots: self.catalog.len(),
            route_slots: self.routes.len(),
            profile_slots,
            co_occurrence_slots: self.co_occurrences.len(),
            pending_pattern_slots: self.pending.len(),
            metadata_source_slots: self.metadata_sources.len(),
            retained_records: 0,
            retained_repository_id_entries: 0,
            per_pattern_repository_sets: 0,
            replay_digest: Some(replay_digest),
        };

        CalibrationRun {
            schema_version: CALIBRATION_SCHEMA_VERSION.to_string(),
            run_id: stable_id("calibration-run", 1),
            dataset_id: metadata.dataset_id,
            pattern_pack: Some(pattern_pack),
            sources,
            summary: CalibrationSummary {
                records: self.records_seen,
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
            resource_trace,
            claim_boundary: default_claim_boundary(),
        }
    }

    fn build_pattern_stats(&self, source_ids: &[String]) -> Vec<PatternStats> {
        let pattern_count = self.catalog.len();
        self.patterns
            .iter()
            .enumerate()
            .filter(|(_, counter)| counter.repositories > 0)
            .map(|(index, counter)| {
                let slot = PatternSlot(index as u16);
                let catalog = self.catalog.pattern(slot);
                let profile_correlations = counter
                    .profile_repositories
                    .iter()
                    .map(|(profile, repositories)| ProfilePatternCorrelation {
                        profile: *profile,
                        repositories: *repositories,
                        frequency_x1000: ratio_x1000(
                            *repositories,
                            *self.profiles.get(profile).unwrap_or(&0),
                        ),
                    })
                    .collect();
                let co_occurrences = (0..pattern_count)
                    .filter_map(|other_index| {
                        let repositories = self.co_occurrences[index * pattern_count + other_index];
                        (repositories > 0).then(|| PatternCoOccurrence {
                            pattern_id: self
                                .catalog
                                .pattern(PatternSlot(other_index as u16))
                                .id
                                .to_string(),
                            repositories,
                            co_frequency_x1000: ratio_x1000(repositories, counter.repositories),
                        })
                    })
                    .collect();
                let confidence = confidence_for(self.records_seen, counter.repositories);
                PatternStats {
                    pattern_id: catalog.id.to_string(),
                    route: catalog.route,
                    repositories: counter.repositories,
                    observations: counter.observations,
                    frequency_x1000: ratio_x1000(counter.repositories, self.records_seen),
                    source_ids: source_ids.to_vec(),
                    profile_correlations,
                    co_occurrences,
                    confidence,
                    confidence_note: confidence_note(
                        self.records_seen,
                        counter.repositories,
                        confidence,
                    ),
                    review_status: CalibrationReviewStatus::PendingReview,
                }
            })
            .collect()
    }

    fn build_route_requirements(&self, source_ids: &[String]) -> Vec<RouteRequirement> {
        self.routes
            .iter()
            .enumerate()
            .map(|(index, (route, counter))| {
                let frequency_x1000 = ratio_x1000(counter.repositories, self.records_seen);
                let confidence = confidence_for(self.records_seen, counter.repositories);
                RouteRequirement {
                    id: stable_id("route-requirement", index + 1),
                    route: *route,
                    supporting_repositories: counter.repositories,
                    observations: counter.observations,
                    frequency_x1000,
                    suggested_requirement: route_requirement_from_frequency(frequency_x1000),
                    priority: priority_from_route_frequency(frequency_x1000),
                    source_ids: source_ids.to_vec(),
                    confidence,
                    review_status: CalibrationReviewStatus::PendingReview,
                    rationale: format!(
                        "Reviewable route requirement candidate only. Route `{:?}` appeared in {} of {} records; maintainers must review source quality and repository purpose before adopting.",
                        route, counter.repositories, self.records_seen
                    ),
                }
            })
            .collect()
    }

    fn build_pending_patterns(&self, source_ids: &[String]) -> Vec<PendingPatternCandidate> {
        self.pending
            .iter()
            .enumerate()
            .map(|(index, (raw_label, counter))| PendingPatternCandidate {
                id: stable_id("pending-pattern", index + 1),
                raw_label: raw_label.clone(),
                observed_repositories: counter.repositories,
                observations: counter.observations,
                source_ids: source_ids.to_vec(),
                example_locations: counter.example_locations.clone(),
                review_status: CalibrationReviewStatus::PendingReview,
            })
            .collect()
    }
}
