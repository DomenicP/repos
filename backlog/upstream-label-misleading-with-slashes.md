---
kind: inconsistency
---

# The upstream label is misleading when branch names contain slashes

`upstream_text` shows only the remote name by removing `/<branch>` from the
end of the upstream. When the upstream is `origin/team/main` and the local
branch is `main`, it shows `origin/team`, which looks like a remote name.
Strip only when the upstream is exactly `<remote>/<branch>`.
