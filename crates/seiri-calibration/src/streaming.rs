mod aggregate;

use self::aggregate::StreamingAccumulator;
use super::CalibrationError;
use seiri_core::{BenchmarkRepoRecord, CalibrationReplayDigest, CalibrationRun};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::num::NonZeroUsize;
use std::path::Path;

const DEFAULT_MAX_LINE_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_MAX_PATTERNS_PER_RECORD: usize = 65_536;
const DEFAULT_MAX_PENDING_PATTERNS: usize = 100_000;
const DEFAULT_MAX_METADATA_SOURCES: usize = 4_096;
const DEFAULT_MAX_TOTAL_INPUT_BYTES: usize = 512 * 1024 * 1024;
const DEFAULT_MAX_RECORDS: usize = 2_000_000;
const DEFAULT_MAX_TOTAL_STRING_BYTES: usize = 512 * 1024 * 1024;
const FNV1A64_OFFSET: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingLimitKind {
    LineBytes,
    PatternsPerRecord,
    PendingPatterns,
    MetadataSources,
    TotalInputBytes,
    Records,
    TotalStringBytes,
}

impl Display for StreamingLimitKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::LineBytes => "line_bytes",
            Self::PatternsPerRecord => "patterns_per_record",
            Self::PendingPatterns => "pending_patterns",
            Self::MetadataSources => "metadata_sources",
            Self::TotalInputBytes => "total_input_bytes",
            Self::Records => "records",
            Self::TotalStringBytes => "total_string_bytes",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamingCalibrationLimits {
    max_line_bytes: NonZeroUsize,
    max_patterns_per_record: NonZeroUsize,
    max_pending_patterns: NonZeroUsize,
    max_metadata_sources: NonZeroUsize,
    max_total_input_bytes: NonZeroUsize,
    max_records: NonZeroUsize,
    max_total_string_bytes: NonZeroUsize,
}

impl StreamingCalibrationLimits {
    #[must_use]
    pub fn new(
        max_line_bytes: usize,
        max_patterns_per_record: usize,
        max_pending_patterns: usize,
        max_metadata_sources: usize,
    ) -> Option<Self> {
        if max_line_bytes == usize::MAX {
            return None;
        }
        Some(Self {
            max_line_bytes: NonZeroUsize::new(max_line_bytes)?,
            max_patterns_per_record: NonZeroUsize::new(max_patterns_per_record)?,
            max_pending_patterns: NonZeroUsize::new(max_pending_patterns)?,
            max_metadata_sources: NonZeroUsize::new(max_metadata_sources)?,
            max_total_input_bytes: NonZeroUsize::new(DEFAULT_MAX_TOTAL_INPUT_BYTES)?,
            max_records: NonZeroUsize::new(DEFAULT_MAX_RECORDS)?,
            max_total_string_bytes: NonZeroUsize::new(DEFAULT_MAX_TOTAL_STRING_BYTES)?,
        })
    }

    #[must_use]
    pub fn with_global_limits(
        mut self,
        max_total_input_bytes: usize,
        max_records: usize,
        max_total_string_bytes: usize,
    ) -> Option<Self> {
        self.max_total_input_bytes = NonZeroUsize::new(max_total_input_bytes)?;
        self.max_records = NonZeroUsize::new(max_records)?;
        self.max_total_string_bytes = NonZeroUsize::new(max_total_string_bytes)?;
        Some(self)
    }

    #[must_use]
    pub fn max_line_bytes(self) -> usize {
        self.max_line_bytes.get()
    }

    #[must_use]
    pub fn max_patterns_per_record(self) -> usize {
        self.max_patterns_per_record.get()
    }

    #[must_use]
    pub fn max_pending_patterns(self) -> usize {
        self.max_pending_patterns.get()
    }

    #[must_use]
    pub fn max_metadata_sources(self) -> usize {
        self.max_metadata_sources.get()
    }

    #[must_use]
    pub fn max_total_input_bytes(self) -> usize {
        self.max_total_input_bytes.get()
    }

    #[must_use]
    pub fn max_records(self) -> usize {
        self.max_records.get()
    }

    #[must_use]
    pub fn max_total_string_bytes(self) -> usize {
        self.max_total_string_bytes.get()
    }
}

impl Default for StreamingCalibrationLimits {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_LINE_BYTES,
            DEFAULT_MAX_PATTERNS_PER_RECORD,
            DEFAULT_MAX_PENDING_PATTERNS,
            DEFAULT_MAX_METADATA_SOURCES,
        )
        .expect("built-in streaming calibration limits are non-zero")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingCalibrationMetadata {
    pub dataset_id: String,
    pub name: String,
    pub collected_at: String,
}

impl StreamingCalibrationMetadata {
    #[must_use]
    pub fn new(
        dataset_id: impl Into<String>,
        name: impl Into<String>,
        collected_at: impl Into<String>,
    ) -> Self {
        Self {
            dataset_id: dataset_id.into(),
            name: name.into(),
            collected_at: collected_at.into(),
        }
    }

    fn from_path(path: &Path) -> Self {
        let dataset_id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("jsonl-dataset")
            .to_string();
        Self::new(dataset_id.clone(), dataset_id, "unknown")
    }
}

pub fn calibrate_jsonl_path(path: impl AsRef<Path>) -> Result<CalibrationRun, CalibrationError> {
    let path = path.as_ref();
    let reader = BufReader::new(File::open(path)?);
    calibrate_jsonl_reader(reader, StreamingCalibrationMetadata::from_path(path))
}

pub fn calibrate_jsonl_reader<R: BufRead>(
    reader: R,
    metadata: StreamingCalibrationMetadata,
) -> Result<CalibrationRun, CalibrationError> {
    calibrate_jsonl_reader_with_limits(reader, metadata, StreamingCalibrationLimits::default())
}

pub fn calibrate_jsonl_reader_with_limits<R: BufRead>(
    mut reader: R,
    metadata: StreamingCalibrationMetadata,
    limits: StreamingCalibrationLimits,
) -> Result<CalibrationRun, CalibrationError> {
    let mut accumulator = StreamingAccumulator::new()?;
    let mut hasher = ReplayHasher::default();
    let mut line = Vec::new();
    let read_cap = limits
        .max_line_bytes()
        .checked_add(1)
        .expect("streaming limit constructor rejects usize::MAX");
    let mut physical_line = 0usize;
    let mut total_input_bytes = 0usize;
    let mut records = 0usize;
    let mut total_string_bytes = 0usize;

    loop {
        line.clear();
        physical_line = physical_line
            .checked_add(1)
            .ok_or(CalibrationError::CounterOverflow {
                line: physical_line,
                counter: "physical_lines",
            })?;
        let read = reader
            .by_ref()
            .take(read_cap as u64)
            .read_until(b'\n', &mut line)?;
        if read == 0 {
            break;
        }
        if line.len() > limits.max_line_bytes() {
            return Err(CalibrationError::StreamingLimitExceeded {
                line: physical_line,
                resource: StreamingLimitKind::LineBytes,
                limit: limits.max_line_bytes(),
                actual: line.len(),
            });
        }
        total_input_bytes = checked_stream_total(
            total_input_bytes,
            line.len(),
            physical_line,
            StreamingLimitKind::TotalInputBytes,
            limits.max_total_input_bytes(),
        )?;

        let text = std::str::from_utf8(&line).map_err(|_| CalibrationError::InvalidUtf8 {
            line: physical_line,
        })?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        records = checked_stream_total(
            records,
            1,
            physical_line,
            StreamingLimitKind::Records,
            limits.max_records(),
        )?;
        let record = serde_json::from_str::<BenchmarkRepoRecord>(trimmed).map_err(|error| {
            CalibrationError::Jsonl {
                line: physical_line,
                message: error.to_string(),
            }
        })?;
        if record.observed_patterns.len() > limits.max_patterns_per_record() {
            return Err(CalibrationError::StreamingLimitExceeded {
                line: physical_line,
                resource: StreamingLimitKind::PatternsPerRecord,
                limit: limits.max_patterns_per_record(),
                actual: record.observed_patterns.len(),
            });
        }
        total_string_bytes = checked_stream_total(
            total_string_bytes,
            record_string_bytes(&record, physical_line)?,
            physical_line,
            StreamingLimitKind::TotalStringBytes,
            limits.max_total_string_bytes(),
        )?;

        hasher.update_record(trimmed.as_bytes());
        accumulator.observe_line(line.len(), record.observed_patterns.len());
        accumulator.push_record(record, physical_line, limits)?;
    }

    Ok(accumulator.finish(metadata, hasher.finish()))
}

fn checked_stream_total(
    current: usize,
    increment: usize,
    line: usize,
    resource: StreamingLimitKind,
    limit: usize,
) -> Result<usize, CalibrationError> {
    let actual = current
        .checked_add(increment)
        .ok_or(CalibrationError::CounterOverflow {
            line,
            counter: "streaming_resource_total",
        })?;
    if actual > limit {
        return Err(CalibrationError::StreamingLimitExceeded {
            line,
            resource,
            limit,
            actual,
        });
    }
    Ok(actual)
}

fn record_string_bytes(
    record: &BenchmarkRepoRecord,
    line: usize,
) -> Result<usize, CalibrationError> {
    let mut strings = std::iter::once(record.repo_id.as_str())
        .chain(record.owner.as_deref())
        .chain(std::iter::once(record.name.as_str()))
        .chain(record.url.as_deref())
        .chain(record.primary_language.as_deref())
        .chain(record.topics.iter().map(String::as_str))
        .chain(std::iter::once(record.metadata_source.as_str()))
        .chain(record.observed_patterns.iter().flat_map(|pattern| {
            pattern
                .pattern_id
                .as_deref()
                .into_iter()
                .chain(std::iter::once(pattern.raw_label.as_str()))
                .chain(pattern.location.as_deref())
        }));
    strings.try_fold(0usize, |total, value| {
        total
            .checked_add(value.len())
            .ok_or(CalibrationError::CounterOverflow {
                line,
                counter: "record_string_bytes",
            })
    })
}

#[derive(Debug)]
struct ReplayHasher(u64);

impl Default for ReplayHasher {
    fn default() -> Self {
        Self(FNV1A64_OFFSET)
    }
}

impl ReplayHasher {
    fn update_record(&mut self, bytes: &[u8]) {
        self.update(&(bytes.len() as u64).to_le_bytes());
        self.update(bytes);
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(FNV1A64_PRIME);
        }
    }

    fn finish(self) -> CalibrationReplayDigest {
        CalibrationReplayDigest::from_u64(self.0)
    }
}
