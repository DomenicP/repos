#![forbid(unsafe_code)]

mod git;
mod render;
mod workspace;

use clap::Parser;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const LEGEND: &str = "Changes: S staged, M modified, ? untracked, ! conflicts;
         subC/subM/sub?/subU: submodule commit/modified/untracked/uninitialized.
Upstream: = equal, +N ahead, -N behind, ? unknown, - absent.
Upstream counts use local refs.";

#[derive(clap::Parser)]
#[command(
    name = "repos",
    args_conflicts_with_subcommands = true,
    after_help = LEGEND
)]
struct Arguments {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    status: StatusArguments,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Summarize local Git repositories
    #[command(after_help = LEGEND)]
    Status(StatusArguments),
}

#[derive(clap::Args)]
struct StatusArguments {
    /// Workspace root (overrides REPOS_ROOT; one is required)
    #[arg(long, value_name = "PATH", allow_hyphen_values = true)]
    root: Option<PathBuf>,
    /// Show only repositories needing attention
    #[arg(long)]
    dirty: bool,
    /// Show submodules as child rows
    #[arg(long)]
    submodules: bool,
    /// Maximum directory depth (root is 0; -1 disables the limit)
    #[arg(long, value_name = "N", default_value_t = 3, allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(-1..))]
    max_depth: i64,
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    match arguments.command {
        Some(Command::Status(options)) => run_status(options),
        None => run_status(arguments.status),
    }
}

fn run_status(options: StatusArguments) -> ExitCode {
    let root = options
        .root
        .or_else(|| env::var_os("REPOS_ROOT").map(PathBuf::from))
        .unwrap_or_default();
    let root = match resolve_root(&root) {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(io::stderr(), "repos: {error}");
            return ExitCode::FAILURE;
        }
    };
    let max_depth = match options.max_depth {
        -1 => None,
        depth => match usize::try_from(depth) {
            Ok(depth) => Some(depth),
            Err(_) => {
                let _ = writeln!(io::stderr(), "repos: --max-depth is too large");
                return ExitCode::from(2);
            }
        },
    };
    let progress = render::Progress::start();
    let (repo_paths, mut errors) = workspace::discover(&root, max_depth);
    let results = workspace::run(&repo_paths, |path| match git::Repository::open(path) {
        Ok(repo) => repo.status(),
        Err(error) => git::Status {
            path: path.to_owned(),
            error: Some(error.message().to_owned()),
            ..git::Status::default()
        },
    });
    let (rows, inspection_errors) = git::collect_status(results);
    errors.extend(inspection_errors);
    drop(progress);

    let color = io::stdout().is_terminal()
        && env::var("TERM").as_deref() != Ok("dumb")
        && env::var_os("NO_COLOR").is_none_or(|value| value.is_empty());
    if let Err(error) = render::table(
        &mut io::stdout().lock(),
        &root,
        &rows,
        options.dirty,
        options.submodules,
        color,
    ) {
        errors.push(format!("output: {error}"));
    }
    let mut stderr = io::stderr().lock();
    for error in &errors {
        let _ = writeln!(stderr, "repos: {error}");
    }
    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

// Resolve only the explicitly selected root. Discovery never follows child symlinks.
fn resolve_root(root: &Path) -> Result<PathBuf, String> {
    if root.as_os_str().is_empty() {
        return Err("provide --root PATH or set REPOS_ROOT".into());
    }
    let resolved = root
        .canonicalize()
        .map_err(|e| format!("root {root:?}: {e}"))?;
    if !resolved.is_dir() {
        return Err(format!("root {root:?} is not a directory"));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[cfg(unix)]
    #[test]
    fn inline_root_preserves_filename_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let mut bytes = b"--root=".to_vec();
        bytes.extend_from_slice(b"repo\xff");
        let arg = OsString::from_vec(bytes);
        let arguments = Arguments::try_parse_from([OsString::from("repos"), arg])
            .expect("test arguments should parse");
        assert_eq!(
            arguments
                .status
                .root
                .expect("explicit root should be present")
                .as_os_str()
                .as_bytes(),
            b"repo\xff"
        );
    }
}
