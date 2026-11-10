---
kind: improvement
---

# Replace the `"(detached)"` sentinel

The literal branch name `"(detached)"` is set in `git/status.rs` and compared
in `git/fast_forward.rs` and twice in `render/status.rs`. Add a `detached: bool`
to `Status` and produce the display text only in render.
