# repos

Bulk commands for quickly working with multiple Git repositories.

## Features

- **status**: Show local Git repositories, branches, changes, and upstream status in a compact table.
- **fetch**: Fetch repositories from remotes in parallel.
- **fast-forward** (alias **ff**): Fetch from remotes, then fast-forward the current branch to its upstream.

## Usage

Requires Rust 1.85+ and a native C toolchain. libgit2 is bundled.

```sh
# In .bashrc or equivalent
export REPOS_ROOT="${HOME}/src"

# Build and install
cargo install --path . --locked

# Run commands
repos status  # default subcommand
repos fetch
repos ff
```

## Development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```
