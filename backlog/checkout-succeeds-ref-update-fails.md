---
kind: bug
---

# Checkout can succeed while the branch update fails

In `fast_forward`, the worktree is checked out before the ref transaction
commits. If the commit fails, the files are at the new commit but the branch is
not, so the repository looks dirty. This is rare, but nothing detects or reports
it.
