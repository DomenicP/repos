---
kind: bug
---

# HTTPS remotes get a misleading credential error

The credentials callback in `git/fetch.rs` only handles SSH agent keys and
usernames. HTTPS remotes usually supply no username, so private HTTPS fetches
fail with "SSH remote must specify a username". Support `USER_PASS_PLAINTEXT`
through `Cred::credential_helper` and make the error match the transport.
