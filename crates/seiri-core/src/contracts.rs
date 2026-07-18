use serde::{Deserialize, Serialize};

pub const ERROR_SCHEMA_VERSION: &str = "seiri.error.v1";
pub const COMPLETION_SCHEMA_VERSION: &str = "seiri.completion.v3";
pub const CONTRACT_SCHEMA_VERSION: &str = "seiri.contract.v3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRevisionKey {
    RepositoryIdentity,
    SourceSession,
    StableDigest,
    MarkdownParser,
    PathClassification,
    DocumentSelection,
    Coverage,
    ContentSlots,
    RouteTarget,
    GithubSemantics,
    DocumentConsistency,
    Profiles,
    ClaimProjection,
    Calibration,
    Delta,
    PatchPlanner,
    Completion,
}

impl SemanticRevisionKey {
    pub const ALL: [Self; 17] = [
        Self::RepositoryIdentity,
        Self::SourceSession,
        Self::StableDigest,
        Self::MarkdownParser,
        Self::PathClassification,
        Self::DocumentSelection,
        Self::Coverage,
        Self::ContentSlots,
        Self::RouteTarget,
        Self::GithubSemantics,
        Self::DocumentConsistency,
        Self::Profiles,
        Self::ClaimProjection,
        Self::Calibration,
        Self::Delta,
        Self::PatchPlanner,
        Self::Completion,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticRevisionEntry<'a> {
    pub key: SemanticRevisionKey,
    pub revision: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticRevisions {
    pub repository_identity: String,
    pub source_session: String,
    pub stable_digest: String,
    pub markdown_parser: String,
    pub path_classification: String,
    pub document_selection: String,
    pub coverage: String,
    pub content_slots: String,
    pub route_target: String,
    pub github_semantics: String,
    pub document_consistency: String,
    pub profiles: String,
    pub claim_projection: String,
    pub calibration: String,
    pub delta: String,
    pub patch_planner: String,
    pub completion: String,
}

impl Default for SemanticRevisions {
    fn default() -> Self {
        Self {
            repository_identity: "seiri.repository-identity.v3".to_string(),
            source_session: "seiri.source-session.v1".to_string(),
            stable_digest: "seiri.stable-digest.v2".to_string(),
            markdown_parser: "seiri.markdown-parser.v3".to_string(),
            path_classification: "seiri.path-classification.v2".to_string(),
            document_selection: "seiri.document-selection.v2".to_string(),
            coverage: "seiri.coverage.v2".to_string(),
            content_slots: "seiri.content-slots.v2".to_string(),
            route_target: "seiri.route-target.v3".to_string(),
            github_semantics: "seiri.github-semantics.v2".to_string(),
            document_consistency: "seiri.document-consistency.v2".to_string(),
            profiles: "seiri.profiles.v2".to_string(),
            claim_projection: crate::CLAIM_SEMANTIC_REVISION.to_string(),
            calibration: "seiri.calibration-semantics.v4".to_string(),
            delta: "seiri.audit-delta-semantics.v3".to_string(),
            patch_planner: "seiri.patch-planner.v4".to_string(),
            completion: "seiri.completion-semantics.v4".to_string(),
        }
    }
}

impl SemanticRevisions {
    #[must_use]
    pub fn entries(&self) -> [SemanticRevisionEntry<'_>; 17] {
        [
            SemanticRevisionEntry {
                key: SemanticRevisionKey::RepositoryIdentity,
                revision: &self.repository_identity,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::SourceSession,
                revision: &self.source_session,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::StableDigest,
                revision: &self.stable_digest,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::MarkdownParser,
                revision: &self.markdown_parser,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::PathClassification,
                revision: &self.path_classification,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::DocumentSelection,
                revision: &self.document_selection,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::Coverage,
                revision: &self.coverage,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::ContentSlots,
                revision: &self.content_slots,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::RouteTarget,
                revision: &self.route_target,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::GithubSemantics,
                revision: &self.github_semantics,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::DocumentConsistency,
                revision: &self.document_consistency,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::Profiles,
                revision: &self.profiles,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::ClaimProjection,
                revision: &self.claim_projection,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::Calibration,
                revision: &self.calibration,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::Delta,
                revision: &self.delta,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::PatchPlanner,
                revision: &self.patch_planner,
            },
            SemanticRevisionEntry {
                key: SemanticRevisionKey::Completion,
                revision: &self.completion,
            },
        ]
    }

    pub fn validate_current(&self) -> Result<(), ContractValidationError> {
        let expected = Self::default();
        for (actual, expected) in self.entries().into_iter().zip(expected.entries()) {
            if actual.key != expected.key || actual.revision != expected.revision {
                return Err(ContractValidationError::SemanticRevision(actual.key));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    InvalidInput,
    Io,
    Contract,
    Internal,
}

impl ErrorClass {
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::InvalidInput => 3,
            Self::Io => 4,
            Self::Contract => 5,
            Self::Internal => 70,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    pub schema_version: String,
    pub class: ErrorClass,
    pub code: String,
    pub message: String,
}

impl ErrorEnvelope {
    #[must_use]
    pub fn new(class: ErrorClass, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema_version: ERROR_SCHEMA_VERSION.to_string(),
            class,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractManifest {
    pub schema_version: String,
    pub tool_version: String,
    pub analysis_schema: String,
    pub patch_plan_schema: String,
    pub codex_schema: String,
    pub error_schema: String,
    pub completion_schema: String,
    pub portable_audit_schema: String,
    pub audit_delta_schema: String,
    pub wording_lint_schema: String,
    pub semantic_revisions: SemanticRevisions,
    pub compatibility: String,
}

impl ContractManifest {
    #[must_use]
    pub fn current(tool_version: impl Into<String>) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
            tool_version: tool_version.into(),
            analysis_schema: crate::ANALYSIS_SCHEMA_VERSION.to_string(),
            patch_plan_schema: crate::PATCH_PLAN_SCHEMA_VERSION.to_string(),
            codex_schema: crate::CODEX_SCHEMA_VERSION.to_string(),
            error_schema: ERROR_SCHEMA_VERSION.to_string(),
            completion_schema: COMPLETION_SCHEMA_VERSION.to_string(),
            portable_audit_schema: crate::PORTABLE_AUDIT_SCHEMA_VERSION.to_string(),
            audit_delta_schema: crate::AUDIT_DELTA_SCHEMA_VERSION.to_string(),
            wording_lint_schema: crate::WORDING_LINT_SCHEMA_VERSION.to_string(),
            semantic_revisions: SemanticRevisions::default(),
            compatibility: "v2-only; v1 inputs, aliases, and silent conversions are rejected"
                .to_string(),
        }
    }

    pub fn validate_current(&self) -> Result<(), ContractValidationError> {
        let expected = Self::current(&self.tool_version);
        if self.schema_version != expected.schema_version {
            return Err(ContractValidationError::ContractSchema);
        }
        if self.analysis_schema != expected.analysis_schema
            || self.patch_plan_schema != expected.patch_plan_schema
            || self.codex_schema != expected.codex_schema
            || self.error_schema != expected.error_schema
            || self.completion_schema != expected.completion_schema
            || self.portable_audit_schema != expected.portable_audit_schema
            || self.audit_delta_schema != expected.audit_delta_schema
            || self.wording_lint_schema != expected.wording_lint_schema
        {
            return Err(ContractValidationError::PublicSchema);
        }
        self.semantic_revisions.validate_current()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractValidationError {
    ContractSchema,
    PublicSchema,
    SemanticRevision(SemanticRevisionKey),
}
