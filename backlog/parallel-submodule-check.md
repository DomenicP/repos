---
kind: improvement
---

# Make the submodule check parallel and cheaper

The submodule filter opens each repository and loads its parent's index one
repository at a time, before the parallel stage. The worker then opens the
repository again. Move the check into `workspace::run`, or cache parent
indexes.
