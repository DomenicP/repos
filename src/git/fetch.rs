use super::Repository;
use git2::{Cred, CredentialType, FetchOptions, RemoteCallbacks};
use std::time::{Duration, Instant};

impl Repository {
    pub fn fetch(&self, mut progress: impl FnMut(&str, &str)) -> (usize, Vec<String>) {
        let remotes = match self.repo.remotes() {
            Ok(remotes) => remotes,
            Err(error) => return (0, vec![error.message().to_owned()]),
        };
        let mut fetched = 0;
        let mut errors = Vec::new();
        for remote in remotes.iter() {
            let remote = match remote {
                Ok(Some(remote)) => remote,
                Ok(None) => {
                    errors.push("remote name is not valid UTF-8".into());
                    continue;
                }
                Err(error) => {
                    errors.push(error.message().to_owned());
                    continue;
                }
            };
            progress(remote, "fetching");
            match self.fetch_remote(remote, &mut progress) {
                Ok(()) => {
                    fetched += 1;
                    progress(remote, "done");
                }
                Err(error) => {
                    progress(remote, "failed");
                    errors.push(format!("remote {remote:?}: {}", error.message()));
                }
            }
        }
        (fetched, errors)
    }

    fn fetch_remote(
        &self,
        name: &str,
        progress: &mut impl FnMut(&str, &str),
    ) -> Result<(), git2::Error> {
        let mut callbacks = RemoteCallbacks::new();
        let mut last_update = Instant::now();
        callbacks.transfer_progress(|stats| {
            // Limit output from parallel transfers to one update per second.
            if last_update.elapsed() >= Duration::from_secs(1) {
                progress(
                    name,
                    &format!(
                        "received {}/{} objects, indexed {} ({:.1} MiB)",
                        stats.received_objects(),
                        stats.total_objects(),
                        stats.indexed_objects(),
                        stats.received_bytes() as f64 / (1024.0 * 1024.0),
                    ),
                );
                last_update = Instant::now();
            }
            true
        });
        let mut agent_attempted = false;
        callbacks.credentials(move |_, username, allowed| {
            let username = username
                .ok_or_else(|| git2::Error::from_str("SSH remote must specify a username"))?;
            if allowed.contains(CredentialType::SSH_KEY) && !agent_attempted {
                // Do not repeatedly offer agent credentials after rejection.
                agent_attempted = true;
                Cred::ssh_key_from_agent(username)
            } else if allowed.contains(CredentialType::USERNAME) {
                Cred::username(username)
            } else {
                Err(git2::Error::from_str(
                    "authentication requires an available SSH agent with an accepted key",
                ))
            }
        });
        let mut options = FetchOptions::new();
        options.remote_callbacks(callbacks);
        self.repo
            .find_remote(name)?
            .fetch(&[] as &[&str], Some(&mut options), None)
    }
}
