---
kind: bug
---

# An unborn submodule breaks its parent's status

`submodule()` in `git/status.rs` calls `repo.head()?`. An initialized
submodule with no commits returns an unborn-branch error, which turns the
whole parent row into an error instead of being counted as a submodule change.
Not yet reproduced.
