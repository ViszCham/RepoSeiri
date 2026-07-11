use crate::RouteKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteContentAtom {
    IdentityPurpose,
    IdentityAudienceOrScope,
    DocsNavigation,
    DocsConceptGuide,
    QuickstartInstallation,
    QuickstartFirstRun,
    SupportQuestionChannel,
    SupportResponseExpectation,
    IntakeReproductionContext,
    IntakeSecurityRedirect,
    ContributingDevelopmentSetup,
    ContributingValidationCommand,
    SecurityDisclosureChannel,
    SecurityPolicyScope,
    ReleaseChangeHistory,
    ReleaseCompatibilityNotes,
    LifecycleMaintenanceStatus,
    LifecycleDeprecationPlan,
    GovernanceDecisionProcess,
    GovernanceMaintainerRole,
    LicenseReference,
    LicenseUsageTerms,
    AutomationWorkflowReference,
    AutomationStatusSignal,
    OwnershipReference,
    OwnershipCriticalPath,
    HygieneGeneratedArtifactPolicy,
    HygieneFormattingPolicy,
}

impl RouteContentAtom {
    pub const ALL: [Self; 28] = [
        Self::IdentityPurpose,
        Self::IdentityAudienceOrScope,
        Self::DocsNavigation,
        Self::DocsConceptGuide,
        Self::QuickstartInstallation,
        Self::QuickstartFirstRun,
        Self::SupportQuestionChannel,
        Self::SupportResponseExpectation,
        Self::IntakeReproductionContext,
        Self::IntakeSecurityRedirect,
        Self::ContributingDevelopmentSetup,
        Self::ContributingValidationCommand,
        Self::SecurityDisclosureChannel,
        Self::SecurityPolicyScope,
        Self::ReleaseChangeHistory,
        Self::ReleaseCompatibilityNotes,
        Self::LifecycleMaintenanceStatus,
        Self::LifecycleDeprecationPlan,
        Self::GovernanceDecisionProcess,
        Self::GovernanceMaintainerRole,
        Self::LicenseReference,
        Self::LicenseUsageTerms,
        Self::AutomationWorkflowReference,
        Self::AutomationStatusSignal,
        Self::OwnershipReference,
        Self::OwnershipCriticalPath,
        Self::HygieneGeneratedArtifactPolicy,
        Self::HygieneFormattingPolicy,
    ];

    #[must_use]
    pub const fn route(self) -> RouteKind {
        match self {
            Self::IdentityPurpose | Self::IdentityAudienceOrScope => RouteKind::Identity,
            Self::DocsNavigation | Self::DocsConceptGuide => RouteKind::Docs,
            Self::QuickstartInstallation | Self::QuickstartFirstRun => RouteKind::Quickstart,
            Self::SupportQuestionChannel | Self::SupportResponseExpectation => RouteKind::Support,
            Self::IntakeReproductionContext | Self::IntakeSecurityRedirect => RouteKind::Intake,
            Self::ContributingDevelopmentSetup | Self::ContributingValidationCommand => {
                RouteKind::Contributing
            }
            Self::SecurityDisclosureChannel | Self::SecurityPolicyScope => RouteKind::Security,
            Self::ReleaseChangeHistory | Self::ReleaseCompatibilityNotes => RouteKind::Release,
            Self::LifecycleMaintenanceStatus | Self::LifecycleDeprecationPlan => {
                RouteKind::Lifecycle
            }
            Self::GovernanceDecisionProcess | Self::GovernanceMaintainerRole => {
                RouteKind::Governance
            }
            Self::LicenseReference | Self::LicenseUsageTerms => RouteKind::License,
            Self::AutomationWorkflowReference | Self::AutomationStatusSignal => {
                RouteKind::Automation
            }
            Self::OwnershipReference | Self::OwnershipCriticalPath => RouteKind::Ownership,
            Self::HygieneGeneratedArtifactPolicy | Self::HygieneFormattingPolicy => {
                RouteKind::Hygiene
            }
        }
    }
}
