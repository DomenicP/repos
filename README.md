# repos

Bulk commands for quickly working with multiple Git repositories. Assumes you already have Git installed.

## Features

**status**: Show local Git repositories, branches, changes, and upstream status in a compact table.

**fetch**: Fetch every configured remote in discovered repositories in parallel using SSH-agent authentication (SSH URLs must include a username). Failed remotes are reported while other fetches continue; any failure exits with code 1. Submodules are fetched only when discovery finds them.

Discovery defaults to depth 3 (root is 0), includes nested repositories and linked worktrees, and skips child directory symlinks.

Changes: `S` staged, `M` modified, `?` untracked, `!` conflicts.
Upstream: `=` equal, `+N` ahead, `-N` behind, `+N/-N` diverged,
`?` unresolved, `-` absent. Counts use local refs and may be stale.

## Usage

Requires Rust 1.85+ and a native C toolchain. libgit2 is bundled.

```sh
# In .bashrc or equiv
export REPOS_ROOT="${HOME}/src"

# Build and install
cargo install --path . --locked

# Run commands
repos status
repos fetch
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

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```
