mod fast_forward;
mod fetch;
mod status;

pub use fast_forward::Outcome;
pub use status::collect_status;

use git2::RepositoryOpenFlags;
use std::path::{Path, PathBuf};

#[derive(Default, Debug, PartialEq)]
pub struct Changes {
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
    pub conflicts: usize,
    pub sub_commits: usize,
    pub sub_modified: usize,
    pub sub_untracked: usize,
    pub uninitialized: usize,
}

impl Changes {
    pub fn attention(&self) -> bool {
        *self != Self::default()
    }
}

#[derive(Default)]
pub struct Status {
    pub path: PathBuf,
    pub parent: Option<PathBuf>,
    pub branch: String,
    pub upstream: String,
    pub no_commits: bool,
    pub uninitialized: bool,
    pub ahead: usize,
    pub behind: usize,
    pub ab_known: bool,
    pub changes: Changes,
    pub error: Option<String>,
    pub submodules: Vec<Submodule>,
}

#[derive(Clone)]
pub struct Submodule {
    pub path: PathBuf,
    pub uninitialized: bool,
}

pub struct Repository {
    repo: git2::Repository,
    path: PathBuf,
}

impl Repository {
    pub fn is_worktree(&self) -> bool {
        self.repo.is_worktree()
    }

    pub fn common_dir(&self) -> &Path {
        self.repo.commondir()
    }

    // A submodule is a checkout whose enclosing repository's index records a
    // gitlink at its path. Its commit is pinned by that parent.
    pub fn is_submodule(&self) -> bool {
        self.path
            .parent()
            .and_then(|dir| git2::Repository::discover(dir).ok())
            .and_then(|parent| {
                let workdir = parent.workdir()?.canonicalize().ok()?;
                let path = self.path.canonicalize().ok()?;
                parent
                    .index()
                    .ok()?
                    .get_path(path.strip_prefix(workdir).ok()?, 0)
            })
            .is_some_and(|entry| entry.mode == u32::from(git2::FileMode::Commit))
    }

    pub fn open(path: &Path) -> Result<Self, git2::Error> {
        let repo = git2::Repository::open_ext(
            path,
            RepositoryOpenFlags::NO_SEARCH,
            std::iter::empty::<&Path>(),
        )?;
        Ok(Self {
            repo,
            path: path.to_owned(),
        })
    }
}
