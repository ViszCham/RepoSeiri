use crate::{RepositoryScopeGraph, ScopeNodeKind, SourceDomain};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryRegion {
    Root,
    WorkspaceMember,
    Submodule,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceUsage {
    Primary,
    Test,
    Fixture,
    Example,
    Generated,
    Vendored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathClassification {
    pub region: RepositoryRegion,
    pub usage: EvidenceUsage,
}

impl PathClassification {
    #[must_use]
    pub fn classify(path: &str, graph: Option<&RepositoryScopeGraph>) -> Self {
        let normalized = normalized_path(path);
        let usage = classify_usage(&normalized);
        let region = graph.map_or_else(
            || fallback_region(&normalized),
            |graph| graph_region(&normalized, graph),
        );
        Self { region, usage }
    }

    #[must_use]
    pub const fn is_primary_repository_content(self) -> bool {
        matches!(
            (self.region, self.usage),
            (
                RepositoryRegion::Root | RepositoryRegion::WorkspaceMember,
                EvidenceUsage::Primary
            )
        )
    }

    #[must_use]
    pub const fn source_domain(self) -> SourceDomain {
        match self.usage {
            EvidenceUsage::Fixture => SourceDomain::Fixture,
            _ => SourceDomain::RepositoryLocal,
        }
    }
}

fn normalized_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

fn classify_usage(path: &str) -> EvidenceUsage {
    for segment in path.split('/') {
        match segment {
            "test" | "tests" | "testdata" | "snapshots" | "benches" => {
                return EvidenceUsage::Test;
            }
            "fixture" | "fixtures" | "__fixtures__" => return EvidenceUsage::Fixture,
            "example" | "examples" | "sample" | "samples" => return EvidenceUsage::Example,
            "target" | "generated" | "dist" | "build" | "coverage" | "node_modules" => {
                return EvidenceUsage::Generated;
            }
            "vendor" | "vendored" | "third_party" | "third-party" => {
                return EvidenceUsage::Vendored;
            }
            _ => {}
        }
    }
    EvidenceUsage::Primary
}

fn fallback_region(path: &str) -> RepositoryRegion {
    if path.starts_with("crates/") || path.starts_with("packages/") {
        RepositoryRegion::WorkspaceMember
    } else {
        RepositoryRegion::Root
    }
}

fn graph_region(path: &str, graph: &RepositoryScopeGraph) -> RepositoryRegion {
    if graph
        .ignored
        .iter()
        .any(|item| path_matches(path, &item.path))
    {
        return RepositoryRegion::Ignored;
    }
    let mut best = None::<(usize, RepositoryRegion)>;
    for node in &graph.nodes {
        let region = match node.kind {
            ScopeNodeKind::Submodule => RepositoryRegion::Submodule,
            ScopeNodeKind::Package if node.path != "." && !node.path.is_empty() => {
                RepositoryRegion::WorkspaceMember
            }
            _ => continue,
        };
        let node_path = normalized_path(&node.path);
        if path_matches(path, &node_path) && best.is_none_or(|(length, _)| node_path.len() > length)
        {
            best = Some((node_path.len(), region));
        }
    }
    best.map_or_else(|| fallback_region(path), |(_, region)| region)
}

fn path_matches(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_axes_cover_supporting_and_vendored_paths() {
        assert_eq!(
            PathClassification::classify("crates/a/tests/case.md", None),
            PathClassification {
                region: RepositoryRegion::WorkspaceMember,
                usage: EvidenceUsage::Test,
            }
        );
        assert_eq!(
            PathClassification::classify("vendor/pkg/README.md", None).usage,
            EvidenceUsage::Vendored
        );
        assert_eq!(
            PathClassification::classify("docs/guide.md", None).usage,
            EvidenceUsage::Primary
        );
    }
}
