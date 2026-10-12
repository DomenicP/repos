mod progress;
mod text;

pub use progress::Progress;

use self::text::{escape, path_text};
use crate::git::{Changes, Status};
use std::io::{self, Write};
use std::path::Path;
use unicode_width::UnicodeWidthStr;

const HEADERS: [&str; 5] = ["REPOSITORY", "BRANCH", "STATE", "CHANGES", "UPSTREAM"];

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

fn state(row: &Status) -> String {
    if row.error.is_some() {
        return "error".into();
    }
    if row.uninitialized {
        return "uninitialized".into();
    }
    let mut states = Vec::new();
    if row.no_commits {
        states.push("no-commits");
    }
    if row.branch == "(detached)" {
        states.push("detached");
    }
    states.push(if row.changes.conflicts > 0 {
        "conflict"
    } else if row.changes.attention() {
        "dirty"
    } else {
        "clean"
    });
    states.join(",")
}

fn upstream_text(row: &Status) -> String {
    if row.upstream.is_empty() {
        return "-".into();
    }
    let suffix = format!("/{}", row.branch);
    let name = if row.branch.is_empty() {
        &row.upstream
    } else {
        row.upstream.strip_suffix(&suffix).unwrap_or(&row.upstream)
    };
    let state = if !row.ab_known {
        "?".into()
    } else if row.ahead > 0 && row.behind > 0 {
        format!("+{}/-{}", row.ahead, row.behind)
    } else if row.ahead > 0 {
        format!("+{}", row.ahead)
    } else if row.behind > 0 {
        format!("-{}", row.behind)
    } else {
        "=".into()
    };
    format!("{} {state}", escape(name))
}

fn changes_text(changes: &Changes) -> String {
    let counts = [
        ("S", changes.staged),
        ("M", changes.modified),
        ("?", changes.untracked),
        ("!", changes.conflicts),
        ("subC", changes.sub_commits),
        ("subM", changes.sub_modified),
        ("sub?", changes.sub_untracked),
        ("subU", changes.uninitialized),
    ];
    let parts: Vec<_> = counts
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(label, n)| format!("{label}:{n}"))
        .collect();
    if parts.is_empty() {
        "-".into()
    } else {
        parts.join(" ")
    }
}

struct TableCell {
    text: String,
    tone: &'static str,
}

impl TableCell {
    fn new(text: String, tone: &'static str) -> Self {
        Self { text, tone }
    }

    fn write(&self, line: &mut String, color: bool, width: Option<usize>) {
        if color && !self.tone.is_empty() {
            line.push_str(self.tone);
            line.push_str(&self.text);
            line.push_str("\x1b[0m");
        } else {
            line.push_str(&self.text);
        }
        if let Some(width) = width {
            line.push_str(&" ".repeat(width - self.text.width() + 1));
        }
    }
}

fn should_show(repo: &Status, dirty: bool, submodules: bool) -> bool {
    if repo.error.is_some() {
        return true;
    }
    if repo.parent.is_some() && !submodules {
        return false;
    }
    !dirty || repo.changes.attention() || repo.uninitialized
}

fn repository_cells(root: &Path, repo: &Status) -> [TableCell; HEADERS.len()] {
    let relative = repo.path.strip_prefix(root).unwrap_or(&repo.path);
    let mut name = path_text(relative);
    if name.is_empty() {
        name = ".".into();
    }
    if repo.parent.is_some() {
        name = format!("  {name}");
    }
    let branch = if repo.branch.is_empty() {
        "-".into()
    } else {
        escape(&repo.branch)
    };

    [
        TableCell::new(name, ""),
        TableCell::new(branch, CYAN),
        state_cell(repo),
        changes_cell(repo),
        upstream_cell(repo),
    ]
}

fn state_cell(repo: &Status) -> TableCell {
    let state_tone = if repo.error.is_some() || repo.changes.conflicts > 0 {
        RED
    } else if repo.no_commits
        || repo.branch == "(detached)"
        || repo.uninitialized
        || repo.changes.attention()
    {
        YELLOW
    } else {
        GREEN
    };

    TableCell::new(state(repo), state_tone)
}

fn changes_cell(repo: &Status) -> TableCell {
    let changes = repo
        .error
        .as_ref()
        .map_or_else(|| changes_text(&repo.changes), |e| escape(e));
    let change_tone = if repo.error.is_some() || repo.changes.conflicts > 0 {
        RED
    } else if changes == "-" {
        DIM
    } else {
        YELLOW
    };
    TableCell::new(changes, change_tone)
}

fn upstream_cell(repo: &Status) -> TableCell {
    let upstream_tone = if repo.upstream.is_empty() {
        DIM
    } else if repo.ahead > 0 && repo.behind > 0 {
        RED
    } else if !repo.ab_known || repo.behind > 0 {
        YELLOW
    } else if repo.ahead > 0 {
        CYAN
    } else {
        GREEN
    };

    TableCell::new(upstream_text(repo), upstream_tone)
}

pub fn table(
    out: &mut impl Write,
    root: &Path,
    rows: &[Status],
    dirty: bool,
    submodules: bool,
    color: bool,
) -> io::Result<()> {
    let mut table = Vec::new();
    for repo in rows {
        if should_show(repo, dirty, submodules) {
            table.push(repository_cells(root, repo));
        }
    }
    if table.is_empty() {
        return writeln!(
            out,
            "{}",
            if rows.is_empty() {
                "No repositories found."
            } else {
                "No repositories need attention."
            }
        );
    }
    table.insert(0, HEADERS.map(|text| TableCell::new(text.into(), BOLD)));
    let mut widths = [0; HEADERS.len()];
    for row in &table {
        for (column, cell) in row.iter().enumerate() {
            widths[column] = widths[column].max(cell.text.width());
        }
    }
    for row in table {
        let mut line = String::new();
        for (column, cell) in row.into_iter().enumerate() {
            cell.write(
                &mut line,
                color,
                (column + 1 < HEADERS.len()).then_some(widths[column]),
            );
        }
        writeln!(out, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_and_distinct_upstream() {
        let row = Status {
            branch: "main".into(),
            upstream: "origin/other".into(),
            ahead: 2,
            behind: 1,
            ab_known: true,
            ..Status::default()
        };
        assert_eq!(upstream_text(&row), "origin/other +2/-1");
        assert_eq!(
            changes_text(&Changes {
                staged: 2,
                untracked: 1,
                sub_untracked: 3,
                ..Changes::default()
            }),
            "S:2 ?:1 sub?:3"
        );
    }

    #[test]
    fn color_preserves_column_spacing() {
        let rows = [Status {
            path: "/root/répo".into(),
            branch: "main".into(),
            ..Status::default()
        }];
        let mut plain = Vec::new();
        let mut colored = Vec::new();
        table(&mut plain, Path::new("/root"), &rows, false, false, false)
            .expect("table should render successfully");
        table(&mut colored, Path::new("/root"), &rows, false, false, true)
            .expect("table should render successfully");
        let mut stripped = String::from_utf8(colored).expect("test output should be valid UTF-8");
        for code in [BOLD, DIM, RED, GREEN, YELLOW, CYAN, "\x1b[0m"] {
            stripped = stripped.replace(code, "");
        }
        assert_eq!(
            String::from_utf8(plain).expect("test output should be valid UTF-8"),
            stripped
        );
    }

    #[test]
    fn wide_unicode_uses_terminal_width() {
        let rows = [Status {
            path: "/root/界".into(),
            branch: "main".into(),
            ..Status::default()
        }];
        let mut output = Vec::new();
        table(&mut output, Path::new("/root"), &rows, false, false, false)
            .expect("table should render successfully");
        assert!(
            String::from_utf8(output)
                .expect("test output should be valid UTF-8")
                .contains("界         main")
        );
    }

    #[test]
    fn propagates_output_failure() {
        struct Failing;
        impl Write for Failing {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(
            table(&mut Failing, Path::new("/root"), &[], false, false, false)
                .expect_err("failing writer should produce an output error")
                .kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}
