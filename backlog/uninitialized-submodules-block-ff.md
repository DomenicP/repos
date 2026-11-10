---
kind: bug
---

# Uninitialized submodules block fast-forward

`Changes::attention()` counts uninitialized submodules, so a repository with an
optional, never-initialized submodule always fails `ff` with "uncommitted
changes". Changes to submodule commits or contents should still block a
fast-forward, but uninitialized submodules do not affect the checkout.

The same count makes `status` report these repositories as "dirty" and always
list them under `--dirty`. Separating uninitialized submodules from
`attention()` fixes both.
