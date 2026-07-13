use crate::DiscoveredRepository;
use gix::bstr::ByteSlice;
use seiri_core::{
    CoverageIncompleteReason, CoverageStatus, GitCommitHeader, GitDiagnostic, GitDiagnosticKind,
    GitObservationState, GitReadBudget, GitReferenceKind, GitReferenceObservation,
    GitTemporalObservation, GitTimestamp,
};
use std::fs;
use std::path::Path;

const MAX_PACKED_REFS_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACK_DIRECTORY_ENTRIES: usize = 4_096;

pub trait GitReadBackend {
    fn observe(&self, root: &DiscoveredRepository, budget: GitReadBudget)
        -> GitTemporalObservation;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GixReadBackend;

impl GitReadBackend for GixReadBackend {
    fn observe(
        &self,
        root: &DiscoveredRepository,
        budget: GitReadBudget,
    ) -> GitTemporalObservation {
        if let Some(diagnostic) = root.diagnostic() {
            return GitTemporalObservation {
                state: GitObservationState::Unknown,
                refs_coverage: partial(CoverageIncompleteReason::ParseFailed),
                tags_coverage: partial(CoverageIncompleteReason::ParseFailed),
                commits_coverage: partial(CoverageIncompleteReason::ParseFailed),
                diagnostics: vec![diagnostic.clone()],
                ..GitTemporalObservation::default()
            };
        }
        let Some(git_dir) = root.git_dir() else {
            return GitTemporalObservation::default();
        };
        let common_dir = root.common_dir().unwrap_or(git_dir);
        let mut observation = GitTemporalObservation {
            state: GitObservationState::Unknown,
            refs_coverage: CoverageStatus::Complete,
            tags_coverage: CoverageStatus::Complete,
            commits_coverage: CoverageStatus::Complete,
            shallow: common_dir.join("shallow").is_file(),
            partial: has_promisor_pack(common_dir),
            ..GitTemporalObservation::default()
        };

        if git_dir.join("objects/info/alternates").is_file()
            || common_dir.join("objects/info/alternates").is_file()
        {
            observation.refs_coverage = partial(CoverageIncompleteReason::UnsupportedSyntax);
            observation.tags_coverage = partial(CoverageIncompleteReason::UnsupportedSyntax);
            observation.commits_coverage = partial(CoverageIncompleteReason::UnsupportedSyntax);
            observation.diagnostics.push(GitDiagnostic {
                kind: GitDiagnosticKind::AlternateObjectDirectoryDisabled,
                path: "objects/info/alternates".to_string(),
            });
            return observation;
        }

        let packed_refs = common_dir.join("packed-refs");
        if packed_refs
            .metadata()
            .is_ok_and(|metadata| metadata.len() > MAX_PACKED_REFS_BYTES)
        {
            observation.refs_coverage = partial(CoverageIncompleteReason::LimitExceeded);
            observation.tags_coverage = partial(CoverageIncompleteReason::LimitExceeded);
            observation.commits_coverage = partial(CoverageIncompleteReason::Unavailable);
            observation.diagnostics.push(GitDiagnostic {
                kind: GitDiagnosticKind::PackedReferencesTooLarge,
                path: "packed-refs".to_string(),
            });
            return observation;
        }

        if observation.shallow {
            observation.diagnostics.push(GitDiagnostic {
                kind: GitDiagnosticKind::ShallowRepository,
                path: "shallow".to_string(),
            });
            observation.commits_coverage = partial(CoverageIncompleteReason::Unavailable);
        }
        if observation.partial {
            observation.diagnostics.push(GitDiagnostic {
                kind: GitDiagnosticKind::PartialRepository,
                path: "objects/pack".to_string(),
            });
            observation.commits_coverage = partial(CoverageIncompleteReason::Unavailable);
        }

        let repository = match gix::open_opts(
            git_dir,
            gix::open::Options::isolated().open_path_as_is(true),
        ) {
            Ok(repository) => repository,
            Err(_) => {
                observation.refs_coverage = partial(CoverageIncompleteReason::ParseFailed);
                observation.tags_coverage = partial(CoverageIncompleteReason::ParseFailed);
                observation.commits_coverage = partial(CoverageIncompleteReason::ParseFailed);
                observation.diagnostics.push(GitDiagnostic {
                    kind: GitDiagnosticKind::Io,
                    path: "git-directory".to_string(),
                });
                return observation;
            }
        };

        observe_head(&repository, &mut observation);
        observe_references(&repository, budget, &mut observation);
        observe_commits(&repository, budget, &mut observation);
        observation.state = GitObservationState::Available;
        observation
    }
}

fn observe_head(repository: &gix::Repository, observation: &mut GitTemporalObservation) {
    match repository.head() {
        Ok(head) => {
            observation.head_name = head
                .referent_name()
                .map(|name| name.as_bstr().to_str_lossy().into_owned());
            observation.head_target = head.id().map(|id| id.to_string());
        }
        Err(_) => observation.diagnostics.push(GitDiagnostic {
            kind: GitDiagnosticKind::MalformedHead,
            path: "HEAD".to_string(),
        }),
    }
}

fn observe_references(
    repository: &gix::Repository,
    budget: GitReadBudget,
    observation: &mut GitTemporalObservation,
) {
    let platform = match repository.references() {
        Ok(platform) => platform,
        Err(_) => {
            observation.refs_coverage = partial(CoverageIncompleteReason::ParseFailed);
            observation.tags_coverage = partial(CoverageIncompleteReason::ParseFailed);
            observation.diagnostics.push(GitDiagnostic {
                kind: GitDiagnosticKind::MalformedReference,
                path: "packed-refs".to_string(),
            });
            return;
        }
    };
    let references = match platform.all() {
        Ok(references) => references,
        Err(_) => {
            observation.refs_coverage = partial(CoverageIncompleteReason::ParseFailed);
            return;
        }
    };
    let mut refs_seen = 0u32;
    let mut tags_seen = 0u32;
    if budget.max_refs == 0 {
        observation.refs_coverage = partial(CoverageIncompleteReason::LimitExceeded);
        observation.tags_coverage = partial(CoverageIncompleteReason::LimitExceeded);
        return;
    }
    for result in references.take(budget.max_refs as usize) {
        let reference = match result {
            Ok(reference) => reference,
            Err(_) => {
                observation.refs_coverage = partial(CoverageIncompleteReason::ParseFailed);
                observation.diagnostics.push(GitDiagnostic {
                    kind: GitDiagnosticKind::MalformedReference,
                    path: "refs".to_string(),
                });
                continue;
            }
        };
        let name = reference.name().as_bstr().to_str_lossy().into_owned();
        let kind = reference_kind(&name);
        refs_seen += 1;
        if kind == GitReferenceKind::Tag {
            if tags_seen >= budget.max_tags {
                observation.tags_coverage = partial(CoverageIncompleteReason::LimitExceeded);
                continue;
            }
            tags_seen += 1;
        }
        let target = match reference.target() {
            gix::refs::TargetRef::Object(id) => id.to_string(),
            gix::refs::TargetRef::Symbolic(name) => name.as_bstr().to_str_lossy().into_owned(),
        };
        observation
            .references
            .push(GitReferenceObservation { name, target, kind });
    }
    if refs_seen == budget.max_refs {
        observation.refs_coverage = partial(CoverageIncompleteReason::LimitExceeded);
    }
    if tags_seen == budget.max_tags && budget.max_tags > 0 {
        observation.tags_coverage = partial(CoverageIncompleteReason::LimitExceeded);
    }
    observation
        .references
        .sort_by(|left, right| left.name.cmp(&right.name));
}

fn observe_commits(
    repository: &gix::Repository,
    budget: GitReadBudget,
    observation: &mut GitTemporalObservation,
) {
    if observation.shallow || observation.partial {
        return;
    }
    let Some(head) = repository.head().ok().and_then(|head| head.id()) else {
        return;
    };
    let walk = repository
        .rev_walk([head.detach()])
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            Default::default(),
        ))
        .use_commit_graph(false)
        .all();
    let Ok(walk) = walk else {
        observation.commits_coverage = partial(CoverageIncompleteReason::ParseFailed);
        return;
    };
    if budget.max_commit_headers == 0 {
        observation.commits_coverage = partial(CoverageIncompleteReason::LimitExceeded);
        return;
    }
    for result in walk.take(budget.max_commit_headers as usize) {
        let info = match result {
            Ok(info) => info,
            Err(_) => {
                observation.commits_coverage = partial(CoverageIncompleteReason::ParseFailed);
                observation.diagnostics.push(GitDiagnostic {
                    kind: GitDiagnosticKind::ObjectDecodeFailed,
                    path: "objects".to_string(),
                });
                break;
            }
        };
        let committed_at = match info.object() {
            Ok(commit) => match commit.time() {
                Ok(time) => GitTimestamp {
                    seconds_since_epoch: time.seconds,
                    offset_minutes: offset_minutes(time.offset),
                },
                Err(_) => {
                    observation.commits_coverage = partial(CoverageIncompleteReason::ParseFailed);
                    continue;
                }
            },
            Err(_) => {
                observation.commits_coverage = partial(CoverageIncompleteReason::ParseFailed);
                continue;
            }
        };
        observation.commits.push(GitCommitHeader {
            object_id: info.id.to_string(),
            committed_at,
        });
    }
    if observation.commits.len() == budget.max_commit_headers as usize {
        observation.commits_coverage = partial(CoverageIncompleteReason::LimitExceeded);
    }
}

fn reference_kind(name: &str) -> GitReferenceKind {
    if name.starts_with("refs/heads/") {
        GitReferenceKind::LocalBranch
    } else if name.starts_with("refs/remotes/") {
        GitReferenceKind::RemoteBranch
    } else if name.starts_with("refs/tags/") {
        GitReferenceKind::Tag
    } else {
        GitReferenceKind::Other
    }
}

fn offset_minutes(offset_seconds: i32) -> i16 {
    i16::try_from(offset_seconds / 60).unwrap_or_else(|_| {
        if offset_seconds.is_negative() {
            i16::MIN
        } else {
            i16::MAX
        }
    })
}

fn has_promisor_pack(common_dir: &Path) -> bool {
    fs::read_dir(common_dir.join("objects/pack"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .take(MAX_PACK_DIRECTORY_ENTRIES)
        .any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|value| value == "promisor")
        })
}

const fn partial(reason: CoverageIncompleteReason) -> CoverageStatus {
    CoverageStatus::Partial(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_repository;
    use gix::bstr::ByteSlice;
    use seiri_core::AnalysisScope;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn gix_backend_reads_bounded_commit_headers_and_offsets() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("reposeiri-gix-{nonce}"));
        let mut repository = gix::init(&root).expect("initialize repository");
        let mut config = repository.config_snapshot_mut();
        config
            .append_config(
                [
                    b"user.name=RepoSeiri".as_bstr(),
                    b"user.email=local@example.invalid".as_bstr(),
                ],
                gix::config::Source::Api,
            )
            .expect("fixture identity config");
        config.commit().expect("apply fixture identity config");
        let tree = repository
            .write_object(gix::objs::Tree::empty())
            .expect("empty tree");
        let signature = gix::actor::SignatureRef {
            name: b"RepoSeiri".as_bstr(),
            email: b"local@example.invalid".as_bstr(),
            time: "1700000000 +0130",
        };
        repository
            .commit_as(
                signature,
                signature,
                "HEAD",
                "fixture",
                tree,
                std::iter::empty::<gix::ObjectId>(),
            )
            .expect("commit");

        let discovered =
            discover_repository(&root, AnalysisScope::Repository).expect("discover repository");
        let observation = GixReadBackend.observe(
            &discovered,
            GitReadBudget {
                max_commit_headers: 1,
                ..GitReadBudget::default()
            },
        );
        assert_eq!(observation.state, GitObservationState::Available);
        assert_eq!(observation.commits.len(), 1);
        assert_eq!(
            observation.commits[0].committed_at.seconds_since_epoch,
            1_700_000_000
        );
        assert_eq!(observation.commits[0].committed_at.offset_minutes, 90);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
