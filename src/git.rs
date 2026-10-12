mod status;

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
