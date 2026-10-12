#![allow(dead_code)] // Each integration test imports only the helpers it needs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

pub struct Fixture {
    pub root: PathBuf,
    _directory: TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("repos-test-")
            .tempdir()
            .expect("fixture directory should be created");
        Self {
            root: directory.path().to_owned(),
            _directory: directory,
        }
    }

    pub fn repo(&self, name: &str, committed: bool) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(&path).expect("fixture directory should be created");
        self.git(&path, &["init", "-b", "main"]);
        for (key, value) in [
            ("user.name", "Fixture"),
            ("user.email", "fixture@example.invalid"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("core.hooksPath", null_device()),
        ] {
            self.git(&path, &["config", key, value]);
        }
        if committed {
            write(&path, "tracked file", "base\n");
            self.commit(&path);
        }
        path
    }

    pub fn git_output(&self, path: &Path, args: &[&str]) -> Output {
        assert!(
            path.starts_with(&self.root),
            "Git fixture must stay inside {:?}",
            self.root
        );
        isolated(Command::new("git"))
            .arg("--no-optional-locks")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .expect("Git must be installed for fixture tests")
    }

    pub fn git(&self, path: &Path, args: &[&str]) -> String {
        let output = self.git_output(path, args);
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("test output should be valid UTF-8")
            .trim()
            .to_owned()
    }

    pub fn commit(&self, path: &Path) {
        self.git(path, &["add", "--all"]);
        self.git(
            path,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "fixture change",
            ],
        );
    }

    pub fn add_submodule(&self, parent: &Path, source: &Path, name: &str) -> PathBuf {
        self.git(
            parent,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                source.to_str().expect("fixture path should be valid UTF-8"),
                name,
            ],
        );
        parent.join(name)
    }
}

pub fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(
        path.parent()
            .expect("fixture file should have a parent directory"),
    )
    .expect("fixture directory should be created");
    fs::write(path, text).expect("fixture file should be written");
}

pub fn command() -> Command {
    let mut command = isolated(Command::new(env!("CARGO_BIN_EXE_repos")));
    command
        .env_remove("REPOS_ROOT")
        .env("TERM", "dumb")
        .env("NO_COLOR", "1");
    command
}

pub fn run(root: &Path, args: &[&str]) -> Output {
    command()
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .expect("repos command should execute")
}

pub fn summary(root: &Path, args: &[&str]) -> String {
    let output = run(root, args);
    assert!(
        output.status.success(),
        "repos {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("test output should be valid UTF-8")
}

fn isolated(mut command: Command) -> Command {
    for name in [
        "GIT_DIR",
        "GIT_COMMON_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
    ] {
        command.env_remove(name);
    }
    command
        .env("GIT_CONFIG_GLOBAL", null_device())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    command
}

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

#[derive(Debug)]
pub struct Row {
    pub name: String,
    pub branch: String,
    pub state: String,
    pub changes: String,
    pub upstream: String,
}

// Parse the public table using its header offsets. These fixtures use ASCII
// display names; wide Unicode alignment has separate rendering unit tests.
pub fn rows(table: &str) -> Vec<Row> {
    let mut lines = table.lines();
    let header = lines.next().expect("table should contain a header");
    if header.starts_with("No repositories") {
        return Vec::new();
    }
    let starts = [
        0,
        header
            .find("BRANCH")
            .expect("table header should contain this column"),
        header
            .find("STATE")
            .expect("table header should contain this column"),
        header
            .find("CHANGES")
            .expect("table header should contain this column"),
        header
            .find("UPSTREAM")
            .expect("table header should contain this column"),
    ];
    lines
        .map(|line| {
            let fields: Vec<_> = starts
                .iter()
                .enumerate()
                .map(|(i, start)| {
                    let end = starts.get(i + 1).copied().unwrap_or(line.len());
                    line[*start..end].trim().to_owned()
                })
                .collect();
            Row {
                name: fields[0].clone(),
                branch: fields[1].clone(),
                state: fields[2].clone(),
                changes: fields[3].clone(),
                upstream: fields[4].clone(),
            }
        })
        .collect()
}

pub fn row(table: &str, name: &str) -> Row {
    rows(table)
        .into_iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("missing {name:?}:\n{table}"))
}
