use super::{Changes, Repository, Status, Submodule};
use git2::{
    Delta, DiffDelta, DiffFindOptions, DiffOptions, ErrorCode, FileMode, Oid, StatusOptions,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

struct Worktree {
    changes: Changes,
    children: Vec<Submodule>,
    tracked: bool,
}

impl Repository {
    // Keep any collected fields alongside a status error so the CLI can
    // still display this repository and collect status for the others.
    pub fn status(&self) -> Status {
        let mut result = Status {
            path: self.path.clone(),
            ..Status::default()
        };
        let status = (|| {
            branch(&self.repo, &mut result)?;
            let worktree = changes(&self.repo, &self.path)?;
            result.changes = worktree.changes;
            result.submodules = worktree.children;
            Ok::<_, git2::Error>(())
        })();
        if let Err(error) = status {
            result.error = Some(error.message().to_owned());
        }
        result
    }
}

fn branch(repo: &git2::Repository, row: &mut Status) -> Result<(), git2::Error> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(e) if e.code() == ErrorCode::UnbornBranch => {
            row.no_commits = true;
            let head = repo.find_reference("HEAD")?;
            row.branch = head
                .symbolic_target()?
                .ok_or_else(|| git2::Error::from_str("unborn HEAD has no symbolic target"))?
                .trim_start_matches("refs/heads/")
                .to_owned();
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    if repo.head_detached()? {
        row.branch = "(detached)".into();
        return Ok(());
    }
    row.branch = head.shorthand()?.to_owned();
    let name = match repo.branch_upstream_name(head.name()?) {
        Ok(name) => name,
        Err(e) if e.code() == ErrorCode::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let name = name.as_str()?;
    row.upstream = name
        .trim_start_matches("refs/remotes/")
        .trim_start_matches("refs/heads/")
        .to_owned();
    let upstream = match repo
        .find_reference(name)
        .and_then(|reference| reference.resolve())
    {
        Ok(upstream) => upstream,
        Err(e) if e.code() == ErrorCode::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    let head_oid = head
        .target()
        .ok_or_else(|| git2::Error::from_str("HEAD has no target"))?;
    let upstream_oid = upstream
        .target()
        .ok_or_else(|| git2::Error::from_str("upstream has no target"))?;
    (row.ahead, row.behind) = repo.graph_ahead_behind(head_oid, upstream_oid)?;
    row.ab_known = true;
    Ok(())
}

fn gitlink(delta: &DiffDelta<'_>) -> bool {
    delta.old_file().mode() == FileMode::Commit || delta.new_file().mode() == FileMode::Commit
}

// Exclude gitlinks here so their staged and worktree changes can be counted
// explicitly, independently of submodule hiding configuration.
fn file_changes(
    repo: &git2::Repository,
    counts: &mut Changes,
) -> Result<HashSet<Vec<u8>>, git2::Error> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .exclude_submodules(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .update_index(false);
    let list = repo.statuses(Some(&mut options))?;
    let mut conflicted = HashSet::new();
    for entry in list.iter() {
        let flags = entry.status();
        if flags.contains(git2::Status::CONFLICTED) {
            counts.conflicts += 1;
            conflicted.insert(entry.path_bytes().to_vec());
            continue;
        }
        if entry.head_to_index().is_some_and(|d| gitlink(&d))
            || entry.index_to_workdir().is_some_and(|d| gitlink(&d))
        {
            continue; // Gitlink changes are counted separately below.
        }
        if flags.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            counts.staged += 1;
        }
        if flags.contains(git2::Status::WT_NEW) {
            counts.untracked += 1;
        }
        if flags.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        ) {
            counts.modified += 1;
        }
    }
    Ok(conflicted)
}

fn staged_gitlinks(
    repo: &git2::Repository,
    index: &git2::Index,
    counts: &mut Changes,
    conflicted: &mut HashSet<Vec<u8>>,
) -> Result<(), git2::Error> {
    let tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        Err(e) if e.code() == ErrorCode::UnbornBranch => None,
        Err(e) => return Err(e),
    };
    let mut options = DiffOptions::new();
    options.ignore_submodules(false);
    let mut diff = repo.diff_tree_to_index(tree.as_ref(), Some(index), Some(&mut options))?;
    diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;
    let mut added: HashMap<Oid, usize> = HashMap::new();
    let mut deleted: HashMap<Oid, usize> = HashMap::new();
    for delta in diff.deltas() {
        if !gitlink(&delta) {
            continue;
        }
        let name = delta
            .new_file()
            .path_bytes()
            .or_else(|| delta.old_file().path_bytes())
            .ok_or_else(|| git2::Error::from_str("staged gitlink has no path"))?
            .to_vec();
        if delta.status() == Delta::Conflicted {
            if conflicted.insert(name) {
                counts.conflicts += 1;
            }
            continue;
        }
        if !conflicted.contains(&name) {
            counts.staged += 1;
        }
        if delta.status() == Delta::Added && delta.new_file().mode() == FileMode::Commit {
            *added.entry(delta.new_file().id()).or_default() += 1;
        }
        if delta.status() == Delta::Deleted && delta.old_file().mode() == FileMode::Commit {
            *deleted.entry(delta.old_file().id()).or_default() += 1;
        }
    }
    for (oid, count) in added {
        // libgit2 reports exact-commit gitlink renames as add/delete. Git counts
        // the pair as one staged path, so subtract the paired deletions.
        counts.staged -= count.min(*deleted.get(&oid).unwrap_or(&0));
    }
    Ok(())
}

fn changes(repo: &git2::Repository, path: &Path) -> Result<Worktree, git2::Error> {
    let mut result = Worktree {
        changes: Changes::default(),
        children: Vec::new(),
        tracked: false,
    };
    let counts = &mut result.changes;
    let mut conflicted = file_changes(repo, counts)?;
    let index = repo.index()?;
    staged_gitlinks(repo, &index, counts, &mut conflicted)?;
    // Keep tracked dirtiness separate from untracked-only child changes. Both
    // contribute to modified counts, but propagate different submodule flags.
    result.tracked = counts.staged > 0 || counts.modified > 0 || counts.conflicts > 0;
    let mut seen = HashSet::new();
    for entry in index.iter() {
        if entry.mode != git2::FileMode::Commit.into() || !seen.insert(entry.path.clone()) {
            continue;
        }
        // Index paths are bytes on Unix; do not silently replace invalid UTF-8.
        let child_path = path.join(path_from_bytes(&entry.path));
        let uninitialized = match child_path.join(".git").symlink_metadata() {
            Ok(_) => false,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(e) => return Err(git2::Error::from_str(&e.to_string())),
        };
        result.children.push(Submodule {
            path: child_path.clone(),
            uninitialized,
        });
        if uninitialized {
            counts.uninitialized += 1;
            continue;
        }
        let state = submodule(&child_path, entry.id).map_err(|error| {
            git2::Error::from_str(&format!("submodule {child_path:?}: {}", error.message()))
        })?;
        result.tracked |= state.commit_changed || state.tracked_changes;
        counts.sub_commits += usize::from(state.commit_changed);
        counts.sub_modified += usize::from(state.tracked_changes);
        counts.sub_untracked += usize::from(state.untracked_changes);
        counts.uninitialized += state.uninitialized;
        if (state.commit_changed || state.tracked_changes || state.untracked_changes)
            && !conflicted.contains(&entry.path)
        {
            counts.modified += 1;
        }
    }
    Ok(result)
}

struct SubmoduleState {
    commit_changed: bool,
    tracked_changes: bool,
    untracked_changes: bool,
    uninitialized: usize,
}

fn submodule(path: &Path, recorded: Oid) -> Result<SubmoduleState, git2::Error> {
    let repo = Repository::open(path)?;
    let head = repo
        .repo
        .head()?
        .target()
        .ok_or_else(|| git2::Error::from_str("submodule HEAD has no target"))?;
    let result = changes(&repo.repo, path)?;
    Ok(SubmoduleState {
        commit_changed: head != recorded,
        tracked_changes: result.tracked,
        untracked_changes: result.changes.untracked > 0 || result.changes.sub_untracked > 0,
        uninitialized: result.changes.uninitialized,
    })
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(bytes).into()
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    String::from_utf8_lossy(bytes).into_owned().into()
}

pub fn collect_status(repos: Vec<Status>) -> (Vec<Status>, Vec<String>) {
    let mut rows = BTreeMap::new();
    let mut relationships = Vec::new();
    let mut errors = Vec::new();
    for repo in repos {
        if let Some(error) = &repo.error {
            errors.push(format!("{:?}: {error}", repo.path));
        }
        relationships.push((repo.path.clone(), repo.submodules.clone()));
        rows.insert(repo.path.clone(), repo);
    }
    // All inspected repositories are now indexed, so a child can be found
    // regardless of which inspection worker finished first.
    for (parent, submodules) in relationships {
        for submodule in submodules {
            if let Some(repo) = rows.get_mut(&submodule.path) {
                // Discovery already found and inspected this submodule. Mark
                // its parent so rendering can hide or indent its row.
                repo.parent = Some(parent.clone());
            } else if submodule.uninitialized {
                // An uninitialized submodule has no checked-out repository
                // to inspect. Add a placeholder so it can still be shown.
                rows.insert(
                    submodule.path.clone(),
                    Status {
                        path: submodule.path,
                        parent: Some(parent.clone()),
                        uninitialized: true,
                        ..Status::default()
                    },
                );
            }
            // Otherwise, the submodule is initialized but was outside the
            // discovery depth. Its changes are already counted in its parent;
            // leave it without a separate row to respect that depth limit.
        }
    }
    errors.sort();
    (rows.into_values().collect(), errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Submodule;
    use std::path::PathBuf;

    #[test]
    fn grouping_is_sorted_and_keeps_only_discovered_initialized_children() {
        let parent = PathBuf::from("/root/parent");
        let discovered = parent.join("child");
        let missing = parent.join("missing");
        let inspections = vec![
            Status {
                path: discovered.clone(),
                ..Status::default()
            },
            Status {
                path: parent.clone(),
                submodules: vec![
                    Submodule {
                        path: discovered.clone(),
                        uninitialized: false,
                    },
                    Submodule {
                        path: parent.join("outside-limit"),
                        uninitialized: false,
                    },
                    Submodule {
                        path: missing.clone(),
                        uninitialized: true,
                    },
                ],
                ..Status::default()
            },
        ];
        let (rows, errors) = collect_status(inspections);
        assert!(errors.is_empty());
        assert_eq!(
            rows.iter().map(|r| &r.path).collect::<Vec<_>>(),
            [&parent, &discovered, &missing]
        );
        assert_eq!(rows[1].parent.as_ref(), Some(&parent));
        assert!(rows[2].uninitialized);
    }
}
