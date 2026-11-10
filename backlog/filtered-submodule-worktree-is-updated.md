---
kind: bug
---

# A filtered submodule's linked worktree is still fast-forwarded

The submodule filter runs before `workspace::fetch_paths`. If a submodule is
filtered out but one of its linked worktrees is in scope, `fetch_paths` picks
that worktree as the representative checkout and updates it.
