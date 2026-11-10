---
kind: bug
---

# The staged count can underflow

In `staged_gitlinks`, the `added` and `deleted` tallies include deltas whose
path is conflicted, but `staged` skips them. The rename correction can then
subtract more than was added, which panics in debug builds and wraps in
release. Not yet reproduced. Tally only non-conflicted deltas, or use
`saturating_sub`.
