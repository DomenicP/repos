---
kind: inconsistency
---

# `ff` updates one checkout per repository

Through `fetch_paths`, `ff` updates one checkout per common git directory.
Branches checked out in other linked worktrees are never fast-forwarded, and
nothing reports that.
