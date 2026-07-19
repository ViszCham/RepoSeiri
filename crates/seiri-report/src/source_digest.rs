use seiri_digest::StableHasher;

pub(crate) fn build_source_session_digest(
    fs_scan: &seiri_fs::RepoFsScan,
    source_store: &seiri_core::SourceStore,
    repository_scope: &seiri_core::RepositoryScopeReport,
) -> seiri_core::SourceSessionDigest {
    let mut hash = StableHasher::new(b"seiri.audit-source-session.v2", 7);
    match &fs_scan.walk_summary.completion {
        seiri_fs::WalkCompletion::Complete => {
            hash.str(1, "complete");
        }
        seiri_fs::WalkCompletion::Truncated(truncation) => {
            hash.str(1, "truncated")
                .str(2, walk_limit_tag(truncation.kind))
                .str(3, &truncation.path)
                .usize(4, truncation.limit);
        }
    }

    hash.usize(5, fs_scan.files.len());
    for file in &fs_scan.files {
        let mut record = StableHasher::new(b"seiri.audit-source-file.v1", 3);
        record
            .str(1, &file.path)
            .u8(2, file_kind_tag(file.kind))
            .u64(3, file.size_bytes);
        hash.digest(6, record.finish());
    }
    hash.usize(7, source_store.documents().len());
    for document in source_store.documents() {
        let mut source = StableHasher::new(b"seiri.audit-source-document.v1", 2);
        source.str(1, document.path()).field(2, document.bytes());
        hash.digest(7, source.finish());
    }
    hash.digest(4, stable_scope_digest(repository_scope));
    seiri_core::SourceSessionDigest::new(hash.finish())
}

const fn file_kind_tag(kind: seiri_core::FileKind) -> u8 {
    match kind {
        seiri_core::FileKind::File => 0,
        seiri_core::FileKind::Directory => 1,
        seiri_core::FileKind::Symlink => 2,
    }
}

fn stable_scope_digest(scope: &seiri_core::RepositoryScopeReport) -> seiri_core::Digest32 {
    let mut hash = StableHasher::new(b"seiri.audit-source-scope.v1", 7);
    hash.u8(1, scope.root.kind as u8)
        .u8(2, scope.root.scope as u8)
        .u8(3, scope.git.state as u8)
        .bool(4, scope.git.shallow)
        .bool(5, scope.git.partial);
    if let Some(head) = &scope.git.head_name {
        hash.str(6, head);
    }
    if let Some(target) = &scope.git.head_target {
        hash.str(7, target);
    }
    for reference in &scope.git.references {
        let mut record = StableHasher::new(b"seiri.audit-source-git-ref.v1", 3);
        record
            .str(1, &reference.name)
            .str(2, &reference.target)
            .u8(3, reference.kind as u8);
        hash.digest(6, record.finish());
    }
    for commit in &scope.git.commits {
        let mut record = StableHasher::new(b"seiri.audit-source-commit.v1", 3);
        record
            .str(1, &commit.object_id)
            .field(2, &commit.committed_at.seconds_since_epoch.to_be_bytes())
            .field(3, &commit.committed_at.offset_minutes.to_be_bytes());
        hash.digest(6, record.finish());
    }
    for node in &scope.graph.nodes {
        let mut record = StableHasher::new(b"seiri.audit-source-scope-node.v1", 4);
        record
            .u32(1, node.id.0)
            .u8(2, node.kind as u8)
            .str(3, &node.path);
        if let Some(manifest) = &node.manifest {
            record.str(4, manifest);
        }
        hash.digest(7, record.finish());
    }
    for edge in &scope.graph.edges {
        let mut record = StableHasher::new(b"seiri.audit-source-scope-edge.v1", 3);
        record
            .u32(1, edge.from.0)
            .u32(2, edge.to.0)
            .u8(3, edge.kind as u8);
        hash.digest(7, record.finish());
    }
    for manifest in &scope.graph.manifests {
        let mut record = StableHasher::new(b"seiri.audit-source-manifest.v1", 5);
        record
            .str(1, &manifest.path)
            .u8(2, manifest.kind as u8)
            .u8(3, manifest.status as u8)
            .bool(4, manifest.declares_workspace)
            .bool(5, manifest.declares_package);
        for member in &manifest.declared_members {
            record.str(5, member);
        }
        hash.digest(7, record.finish());
    }
    for ignored in &scope.graph.ignored {
        let mut record = StableHasher::new(b"seiri.audit-source-ignored.v1", 3);
        record
            .str(1, &ignored.path)
            .u8(2, file_kind_tag(ignored.kind))
            .u8(3, ignored.reason as u8);
        hash.digest(7, record.finish());
    }
    hash.finish()
}

const fn walk_limit_tag(kind: seiri_fs::WalkLimitKind) -> &'static str {
    match kind {
        seiri_fs::WalkLimitKind::Depth => "depth",
        seiri_fs::WalkLimitKind::Entries => "entries",
        seiri_fs::WalkLimitKind::DirectoryEntries => "directory_entries",
    }
}
