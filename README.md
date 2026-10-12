# repos

Bulk commands for quickly working with multiple Git repositories.

## Features

**status**: Show local Git repositories, branches, changes, and upstream status in a compact table.

Discovery defaults to depth 3 (root is 0), includes nested repositories and linked worktrees, and skips child directory symlinks.

Changes: `S` staged, `M` modified, `?` untracked, `!` conflicts.
Upstream: `=` equal, `+N` ahead, `-N` behind, `+N/-N` diverged,
`?` unresolved, `-` absent. Counts use local refs and may be stale.

## Usage

Requires Rust 1.85+ and a native C toolchain. libgit2 is bundled.

```sh
cargo install --path . --locked
repos status --root ~/src
```

`repos` defaults to `status`. Provide the workspace root with `--root` or
`REPOS_ROOT`; `--root` takes precedence.

```sh
repos --root ~/src --dirty          # Only repositories needing attention
repos --root ~/src --submodules     # Include submodule rows
repos --root ~/src --max-depth -1   # Search without a depth limit
repos --help                        # All options and output labels
```

## Development

Tests require Git.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```
