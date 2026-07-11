use seiri_core::{
    CoverageIncompleteReason, CoverageStatus, FileKind, FileRecord, IgnoredShallowRecord,
    ManifestObservationStatus, RepositoryScopeGraph, ScopeEdge, ScopeEdgeKind, ScopeNode,
    ScopeNodeId, ScopeNodeKind, ScopeReadBudget, WorkspaceManifestKind,
    WorkspaceManifestObservation,
};
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

pub(crate) fn build_scope_graph(
    root: &Path,
    files: &[FileRecord],
    ignored: &[IgnoredShallowRecord],
    ignored_truncated: bool,
    budget: ScopeReadBudget,
) -> RepositoryScopeGraph {
    if budget.max_nodes == 0 {
        return RepositoryScopeGraph {
            node_coverage: partial(CoverageIncompleteReason::LimitExceeded),
            manifest_coverage: CoverageStatus::NotRequested,
            ignored_coverage: CoverageStatus::NotRequested,
            ..RepositoryScopeGraph::default()
        };
    }
    let mut builder = GraphBuilder::new(budget.max_nodes);
    let repository = builder
        .add_node(ScopeNodeKind::Repository, String::new(), None)
        .expect("non-zero default and explicit node budgets are validated by truncation");
    let mut manifests = scan_manifests(root, files, budget.max_manifest_bytes);
    let manifest_coverage = manifest_coverage(&manifests);

    for manifest in &manifests {
        let parent = parent_path(&manifest.path);
        let manifest_name = Some(manifest.path.clone());
        if manifest.declares_workspace {
            if let Some(node) = builder.add_node(
                ScopeNodeKind::Workspace,
                parent.clone(),
                manifest_name.clone(),
            ) {
                builder.add_edge(repository, node, ScopeEdgeKind::Contains);
            }
        }
        if manifest.declares_package {
            if let Some(node) =
                builder.add_node(ScopeNodeKind::Package, parent, manifest_name.clone())
            {
                builder.add_edge(repository, node, ScopeEdgeKind::Contains);
                builder.add_edge(repository, node, ScopeEdgeKind::PackageManifest);
            }
        }
    }

    for file in files {
        if file.kind == FileKind::File && file.path.ends_with("go.mod") {
            let path = parent_path(&file.path);
            if let Some(node) =
                builder.add_node(ScopeNodeKind::Package, path, Some(file.path.clone()))
            {
                builder.add_edge(repository, node, ScopeEdgeKind::Contains);
            }
        }
        if file.kind != FileKind::Directory {
            continue;
        }
        let (kind, edge) = match final_component(&file.path) {
            "docs" | "documentation" => {
                (ScopeNodeKind::Documentation, ScopeEdgeKind::Documentation)
            }
            "example" | "examples" => (ScopeNodeKind::Example, ScopeEdgeKind::Example),
            "fixture" | "fixtures" => (ScopeNodeKind::Fixture, ScopeEdgeKind::Fixture),
            _ => continue,
        };
        if let Some(node) = builder.add_node(kind, file.path.clone(), None) {
            let owner = builder.closest_container(&file.path).unwrap_or(repository);
            builder.add_edge(owner, node, edge);
        }
    }

    add_declared_member_edges(&mut builder, &manifests, repository);
    add_submodule_nodes(
        root,
        files,
        budget.max_manifest_bytes,
        &mut builder,
        repository,
    );

    let ignored_limit = budget.max_ignored_records as usize;
    let retained_ignored = ignored
        .iter()
        .take(ignored_limit)
        .cloned()
        .collect::<Vec<_>>();
    let ignored_coverage = if ignored_truncated || ignored.len() > ignored_limit {
        partial(CoverageIncompleteReason::LimitExceeded)
    } else {
        CoverageStatus::Complete
    };
    manifests.sort_by(|left, right| left.path.cmp(&right.path));
    let (nodes, edges, node_coverage) = builder.finish();
    RepositoryScopeGraph {
        nodes,
        edges,
        manifests,
        ignored: retained_ignored,
        node_coverage,
        manifest_coverage,
        ignored_coverage,
        ..RepositoryScopeGraph::default()
    }
}

struct GraphBuilder {
    max_nodes: u32,
    truncated: bool,
    nodes: Vec<ScopeNode>,
    keys: BTreeMap<(u8, String), ScopeNodeId>,
    edges: BTreeSet<(ScopeNodeId, ScopeNodeId, u8)>,
}

impl GraphBuilder {
    fn new(max_nodes: u32) -> Self {
        Self {
            max_nodes,
            truncated: false,
            nodes: Vec::new(),
            keys: BTreeMap::new(),
            edges: BTreeSet::new(),
        }
    }

    fn add_node(
        &mut self,
        kind: ScopeNodeKind,
        path: String,
        manifest: Option<String>,
    ) -> Option<ScopeNodeId> {
        let key = (node_rank(kind), path.clone());
        if let Some(id) = self.keys.get(&key) {
            return Some(*id);
        }
        if self.nodes.len() >= self.max_nodes as usize {
            self.truncated = true;
            return None;
        }
        let id = ScopeNodeId(self.nodes.len() as u32 + 1);
        self.nodes.push(ScopeNode {
            id,
            kind,
            path,
            manifest,
        });
        self.keys.insert(key, id);
        Some(id)
    }

    fn add_edge(&mut self, from: ScopeNodeId, to: ScopeNodeId, kind: ScopeEdgeKind) {
        if from != to {
            self.edges.insert((from, to, edge_rank(kind)));
        }
    }

    fn closest_container(&self, path: &str) -> Option<ScopeNodeId> {
        self.nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.kind,
                    ScopeNodeKind::Repository | ScopeNodeKind::Workspace | ScopeNodeKind::Package
                ) && contains_path(&node.path, path)
            })
            .max_by_key(|node| node.path.len())
            .map(|node| node.id)
    }

    fn node_id(&self, kind: ScopeNodeKind, path: &str) -> Option<ScopeNodeId> {
        self.keys.get(&(node_rank(kind), path.to_string())).copied()
    }

    fn finish(mut self) -> (Vec<ScopeNode>, Vec<ScopeEdge>, CoverageStatus) {
        self.nodes.sort_by_key(|node| node.id);
        let edges = self
            .edges
            .into_iter()
            .map(|(from, to, rank)| ScopeEdge {
                from,
                to,
                kind: edge_from_rank(rank),
            })
            .collect();
        let coverage = if self.truncated {
            partial(CoverageIncompleteReason::LimitExceeded)
        } else {
            CoverageStatus::Complete
        };
        (self.nodes, edges, coverage)
    }
}

fn scan_manifests(
    root: &Path,
    files: &[FileRecord],
    max_bytes: u64,
) -> Vec<WorkspaceManifestObservation> {
    files
        .iter()
        .filter(|file| file.kind == FileKind::File)
        .filter_map(|file| manifest_kind(&file.path).map(|kind| (file, kind)))
        .map(|(file, kind)| read_manifest(root, file, kind, max_bytes))
        .collect()
}

fn manifest_kind(path: &str) -> Option<WorkspaceManifestKind> {
    Some(match final_component(path) {
        "Cargo.toml" => WorkspaceManifestKind::Cargo,
        "package.json" => WorkspaceManifestKind::Npm,
        "pyproject.toml" => WorkspaceManifestKind::Pyproject,
        "go.work" => WorkspaceManifestKind::GoWork,
        _ => return None,
    })
}

fn read_manifest(
    root: &Path,
    file: &FileRecord,
    kind: WorkspaceManifestKind,
    max_bytes: u64,
) -> WorkspaceManifestObservation {
    if file.size_bytes > max_bytes {
        return manifest(file, kind, ManifestObservationStatus::SourceTooLarge, None);
    }
    let path = root.join(path_from_slashes(&file.path));
    let handle = match fs::File::open(path) {
        Ok(handle) => handle,
        Err(_) => return manifest(file, kind, ManifestObservationStatus::Malformed, None),
    };
    let mut bytes = Vec::new();
    match handle
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
    {
        Ok(_) if bytes.len() as u64 > max_bytes => {
            return manifest(file, kind, ManifestObservationStatus::SourceTooLarge, None)
        }
        Ok(_) => {}
        Err(_) => return manifest(file, kind, ManifestObservationStatus::Malformed, None),
    }
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => return manifest(file, kind, ManifestObservationStatus::InvalidUtf8, None),
    };
    let parsed = match kind {
        WorkspaceManifestKind::Cargo => cargo_members(text),
        WorkspaceManifestKind::Npm => npm_members(text),
        WorkspaceManifestKind::Pyproject => pyproject_members(text),
        WorkspaceManifestKind::GoWork => go_work_members(text),
    };
    match parsed {
        Some(parsed) => manifest(file, kind, ManifestObservationStatus::Parsed, Some(parsed)),
        None => manifest(file, kind, ManifestObservationStatus::Malformed, None),
    }
}

struct ParsedManifest {
    members: Vec<String>,
    declares_workspace: bool,
    declares_package: bool,
}

fn manifest(
    file: &FileRecord,
    kind: WorkspaceManifestKind,
    status: ManifestObservationStatus,
    parsed: Option<ParsedManifest>,
) -> WorkspaceManifestObservation {
    let (mut declared_members, declares_workspace, declares_package) = parsed.map_or_else(
        || (Vec::new(), false, false),
        |parsed| {
            (
                parsed.members,
                parsed.declares_workspace,
                parsed.declares_package,
            )
        },
    );
    declared_members.sort();
    declared_members.dedup();
    WorkspaceManifestObservation {
        path: file.path.clone(),
        kind,
        status,
        declares_workspace,
        declares_package,
        declared_members,
    }
}

fn cargo_members(text: &str) -> Option<ParsedManifest> {
    let value = text.parse::<TomlValue>().ok()?;
    let workspace = value.get("workspace");
    Some(ParsedManifest {
        members: toml_string_array(workspace.and_then(|value| value.get("members"))),
        declares_workspace: workspace.is_some(),
        declares_package: value.get("package").is_some(),
    })
}

fn pyproject_members(text: &str) -> Option<ParsedManifest> {
    let value = text.parse::<TomlValue>().ok()?;
    let workspace = value
        .get("tool")
        .and_then(|value| value.get("uv"))
        .and_then(|value| value.get("workspace"));
    Some(ParsedManifest {
        members: toml_string_array(workspace.and_then(|value| value.get("members"))),
        declares_workspace: workspace.is_some(),
        declares_package: value.get("project").is_some(),
    })
}

fn npm_members(text: &str) -> Option<ParsedManifest> {
    let value = serde_json::from_str::<JsonValue>(text).ok()?;
    let workspaces = value.get("workspaces");
    let members = match workspaces {
        Some(JsonValue::Array(values)) => json_strings(values),
        Some(JsonValue::Object(object)) => object
            .get("packages")
            .and_then(JsonValue::as_array)
            .map_or_else(Vec::new, |values| json_strings(values)),
        None => Vec::new(),
        _ => return None,
    };
    Some(ParsedManifest {
        members,
        declares_workspace: workspaces.is_some(),
        declares_package: value.get("name").is_some(),
    })
}

fn go_work_members(text: &str) -> Option<ParsedManifest> {
    let mut members = Vec::new();
    let mut in_use = false;
    for raw in text.lines() {
        let line = raw.split("//").next().unwrap_or_default().trim();
        if line == "use (" {
            in_use = true;
            continue;
        }
        if in_use && line == ")" {
            in_use = false;
            continue;
        }
        if let Some(value) = line.strip_prefix("use ") {
            members.push(value.trim_matches('"').to_string());
        } else if in_use && !line.is_empty() {
            members.push(line.trim_matches('"').to_string());
        }
    }
    (!in_use).then_some(ParsedManifest {
        members,
        declares_workspace: true,
        declares_package: false,
    })
}

fn toml_string_array(value: Option<&TomlValue>) -> Vec<String> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn json_strings(values: &[JsonValue]) -> Vec<String> {
    values
        .iter()
        .filter_map(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn add_declared_member_edges(
    builder: &mut GraphBuilder,
    manifests: &[WorkspaceManifestObservation],
    repository: ScopeNodeId,
) {
    for manifest in manifests {
        let workspace_path = parent_path(&manifest.path);
        let workspace = builder
            .node_id(ScopeNodeKind::Workspace, &workspace_path)
            .unwrap_or(repository);
        let package_paths = builder
            .nodes
            .iter()
            .filter(|node| node.kind == ScopeNodeKind::Package)
            .map(|node| (node.id, node.path.clone()))
            .collect::<Vec<_>>();
        for member in &manifest.declared_members {
            let normalized = join_relative(&workspace_path, member);
            for (package, path) in &package_paths {
                if member_matches(&normalized, path) {
                    builder.add_edge(workspace, *package, ScopeEdgeKind::DeclaresMember);
                }
            }
        }
    }
}

fn add_submodule_nodes(
    root: &Path,
    files: &[FileRecord],
    max_bytes: u64,
    builder: &mut GraphBuilder,
    repository: ScopeNodeId,
) {
    let Some(file) = files.iter().find(|file| file.path == ".gitmodules") else {
        return;
    };
    if file.size_bytes > max_bytes {
        return;
    }
    let Ok(handle) = fs::File::open(root.join(".gitmodules")) else {
        return;
    };
    let mut bytes = Vec::new();
    if handle
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > max_bytes
    {
        return;
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    for line in text.lines() {
        let Some(path) = line.trim().strip_prefix("path =") else {
            continue;
        };
        let path = normalize_member(path.trim());
        if let Some(node) = builder.add_node(ScopeNodeKind::Submodule, path, None) {
            builder.add_edge(repository, node, ScopeEdgeKind::SubmoduleBoundary);
        }
    }
}

fn manifest_coverage(manifests: &[WorkspaceManifestObservation]) -> CoverageStatus {
    if manifests
        .iter()
        .any(|manifest| manifest.status == ManifestObservationStatus::SourceTooLarge)
    {
        partial(CoverageIncompleteReason::LimitExceeded)
    } else if manifests.iter().any(|manifest| {
        matches!(
            manifest.status,
            ManifestObservationStatus::InvalidUtf8
                | ManifestObservationStatus::Malformed
                | ManifestObservationStatus::Unsupported
        )
    }) {
        partial(CoverageIncompleteReason::ParseFailed)
    } else {
        CoverageStatus::Complete
    }
}

fn member_matches(pattern: &str, path: &str) -> bool {
    let pattern_segments = pattern.split('/').collect::<Vec<_>>();
    let path_segments = path.split('/').collect::<Vec<_>>();
    pattern_segments.len() == path_segments.len()
        && pattern_segments
            .iter()
            .zip(path_segments)
            .all(|(left, right)| *left == "*" || *left == right)
}

fn contains_path(parent: &str, child: &str) -> bool {
    parent.is_empty()
        || child == parent
        || child
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn parent_path(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(parent, _)| parent.to_string())
}

fn final_component(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn join_relative(parent: &str, member: &str) -> String {
    let member = normalize_member(member);
    if parent.is_empty() {
        member
    } else {
        format!("{parent}/{member}")
    }
}

fn normalize_member(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn path_from_slashes(path: &str) -> PathBuf {
    path.split('/').collect()
}

const fn node_rank(kind: ScopeNodeKind) -> u8 {
    match kind {
        ScopeNodeKind::Repository => 0,
        ScopeNodeKind::Workspace => 1,
        ScopeNodeKind::Package => 2,
        ScopeNodeKind::Documentation => 3,
        ScopeNodeKind::Example => 4,
        ScopeNodeKind::Fixture => 5,
        ScopeNodeKind::Submodule => 6,
    }
}

const fn edge_rank(kind: ScopeEdgeKind) -> u8 {
    match kind {
        ScopeEdgeKind::Contains => 0,
        ScopeEdgeKind::DeclaresMember => 1,
        ScopeEdgeKind::PackageManifest => 2,
        ScopeEdgeKind::Documentation => 3,
        ScopeEdgeKind::Example => 4,
        ScopeEdgeKind::Fixture => 5,
        ScopeEdgeKind::SubmoduleBoundary => 6,
    }
}

const fn edge_from_rank(rank: u8) -> ScopeEdgeKind {
    match rank {
        0 => ScopeEdgeKind::Contains,
        1 => ScopeEdgeKind::DeclaresMember,
        2 => ScopeEdgeKind::PackageManifest,
        3 => ScopeEdgeKind::Documentation,
        4 => ScopeEdgeKind::Example,
        5 => ScopeEdgeKind::Fixture,
        _ => ScopeEdgeKind::SubmoduleBoundary,
    }
}

const fn partial(reason: CoverageIncompleteReason) -> CoverageStatus {
    CoverageStatus::Partial(reason)
}
