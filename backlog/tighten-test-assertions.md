---
kind: improvement
---

# Tighten test assertions

Pedantic clippy flags `assert!(x.is_empty())` in tests. Use `assert_eq!` so a
failing test shows the value. `tests/performance.rs` should use
`map_or_else`.
