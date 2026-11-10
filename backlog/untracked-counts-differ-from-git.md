---
kind: inconsistency
---

# Untracked counts differ from `git status`

`recurse_untracked_dirs(true)` counts every file in an untracked directory,
while `git status` shows one entry. It is also slow on large untracked trees.
