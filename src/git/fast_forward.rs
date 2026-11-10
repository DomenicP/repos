use super::Repository;
use git2::{RepositoryState, build::CheckoutBuilder};

pub enum Outcome {
    Done(&'static str),
    /// Nothing was attempted; holds the reason.
    Skipped(&'static str),
}

impl Repository {
    pub fn fast_forward(&self) -> Result<Outcome, git2::Error> {
        if self.repo.state() != RepositoryState::Clean {
            return Err(git2::Error::from_str("repository operation in progress"));
        }
        let status = self.status();
        if let Some(error) = status.error {
            return Err(git2::Error::from_str(&error));
        }
        if status.no_commits {
            return Ok(Outcome::Skipped("no commits"));
        }
        if status.branch == "(detached)" {
            return Ok(Outcome::Skipped("detached HEAD"));
        }
        if status.upstream.is_empty() {
            return Ok(Outcome::Skipped("no upstream"));
        }
        // Status leaves this unset only when the configured upstream ref does
        // not exist, typically because the remote branch was deleted and pruned.
        if !status.ab_known {
            return Ok(Outcome::Skipped("upstream gone"));
        }
        if status.behind == 0 {
            return Ok(Outcome::Done(if status.ahead == 0 {
                "up to date"
            } else {
                "ahead of upstream"
            }));
        }
        if status.ahead > 0 {
            return Err(git2::Error::from_str(
                "branches have diverged; cannot fast-forward",
            ));
        }
        if status.changes.attention() {
            return Err(git2::Error::from_str(
                "uncommitted changes; cannot fast-forward",
            ));
        }

        let head = self.repo.head()?;
        let branch = head.name()?;
        let mut transaction = self.repo.transaction()?;
        transaction.lock_ref("HEAD")?;
        transaction.lock_ref(branch)?;
        let locked_head = self.repo.head()?;
        if locked_head.name()? != branch || locked_head.target() != head.target() {
            return Err(git2::Error::from_str(
                "HEAD changed during fast-forward; retry",
            ));
        }
        let upstream = self.repo.branch_upstream_name(branch)?;
        let target = self
            .repo
            .find_reference(upstream.as_str()?)?
            .peel_to_commit()?;
        let current = head.peel_to_commit()?;
        let (ahead, _) = self.repo.graph_ahead_behind(current.id(), target.id())?;
        if ahead > 0 {
            return Err(git2::Error::from_str(
                "upstream changed; cannot fast-forward",
            ));
        }
        // Checkout writes the index after updating files. Refuse an existing
        // index lock before touching the worktree.
        if self
            .repo
            .path()
            .join("index.lock")
            .try_exists()
            .map_err(|error| git2::Error::from_str(&format!("checking index lock: {error}")))?
        {
            return Err(git2::Error::from_str(
                "index is locked; cannot fast-forward",
            ));
        }
        transaction.set_target(branch, target.id(), None, "repos ff: fast-forward")?;
        let mut checkout = CheckoutBuilder::new();
        checkout.safe().overwrite_ignored(false);
        self.repo
            .checkout_tree(target.as_object(), Some(&mut checkout))?;
        transaction.commit()?;
        Ok(Outcome::Done("fast-forwarded"))
    }
}
