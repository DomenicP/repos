mod support;
use std::path::{Path, PathBuf};
use std::process::Output;
use support::*;

fn fast_forward(root: &Path, name: &str) -> Output {
    command()
        .arg(name)
        .arg("--root")
        .arg(root)
        .output()
        .expect("fast-forward command should execute")
}

fn tracking_repo(fixture: &Fixture, name: &str, source: &Path) -> PathBuf {
    let local = fixture.repo(name, false);
    fixture.git(
        &local,
        &[
            "remote",
            "add",
            "origin",
            source.to_str().expect("UTF-8 path"),
        ],
    );
    fixture.git(&local, &["fetch", "origin"]);
    fixture.git(&local, &["reset", "--hard", "origin/main"]);
    fixture.git(&local, &["branch", "--set-upstream-to=origin/main"]);
    local
}

fn head(fixture: &Fixture, path: &Path) -> String {
    fixture.git(path, &["rev-parse", "HEAD"])
}

#[test]
fn fetches_before_fast_forward_and_supports_alias() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let local = tracking_repo(&fixture, "workspace/local", &source);
    for name in ["fast-forward", "ff"] {
        let previous = head(&fixture, &local);
        write(&source, "tracked file", name);
        fixture.commit(&source);
        assert_ne!(head(&fixture, &source), previous);
        assert_eq!(fixture.git(&local, &["rev-parse", "origin/main"]), previous);
        let output = fast_forward(&fixture.root.join("workspace"), name);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "✓ local: fast-forwarded"
        );
        assert!(output.stderr.is_empty());
        assert_eq!(head(&fixture, &local), head(&fixture, &source));
        assert_eq!(
            std::fs::read_to_string(local.join("tracked file")).expect("file"),
            name
        );
        assert!(fixture.git(&local, &["status", "--porcelain"]).is_empty());
    }
    let output = fast_forward(&local, "ff");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("up to date"));
    write(&local, "local file", "local commit\n");
    fixture.commit(&local);
    let before = head(&fixture, &local);
    let output = fast_forward(&local, "ff");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("ahead of upstream"));
    assert_eq!(head(&fixture, &local), before);
}

#[test]
fn dirty_repositories_are_unchanged_while_other_repositories_advance() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let good = tracking_repo(&fixture, "workspace/good", &source);
    let modified = tracking_repo(&fixture, "workspace/modified", &source);
    let staged = tracking_repo(&fixture, "workspace/staged", &source);
    let untracked = tracking_repo(&fixture, "workspace/untracked", &source);
    let before = head(&fixture, &good);
    write(&modified, "tracked file", "local edits\n");
    write(&staged, "tracked file", "staged edits\n");
    fixture.git(&staged, &["add", "tracked file"]);
    write(&untracked, "untracked file", "keep me\n");
    write(&source, "tracked file", "remote edits\n");
    fixture.commit(&source);
    let dirty = [&modified, &staged, &untracked];
    let states: Vec<_> = dirty
        .iter()
        .map(|repo| fixture.git(repo, &["status", "--porcelain"]))
        .collect();
    let output = fast_forward(&fixture.root.join("workspace"), "ff");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("uncommitted changes"));
    assert_eq!(head(&fixture, &good), head(&fixture, &source));
    for (repo, state) in dirty.into_iter().zip(states) {
        assert_eq!(head(&fixture, repo), before);
        assert_eq!(fixture.git(repo, &["status", "--porcelain"]), state);
    }
    assert_eq!(
        std::fs::read_to_string(modified.join("tracked file")).expect("file"),
        "local edits\n"
    );
    assert_eq!(
        std::fs::read_to_string(untracked.join("untracked file")).expect("file"),
        "keep me\n"
    );
}

#[test]
fn refuses_divergence_and_fetch_failure_without_moving_head() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let diverged = tracking_repo(&fixture, "workspace/diverged", &source);
    let broken = tracking_repo(&fixture, "workspace/broken", &source);
    write(&diverged, "local file", "local\n");
    fixture.commit(&diverged);
    let diverged_before = head(&fixture, &diverged);
    let broken_before = head(&fixture, &broken);
    fixture.git(
        &broken,
        &[
            "remote",
            "add",
            "bad",
            fixture.root.join("absent").to_str().expect("UTF-8 path"),
        ],
    );
    write(&source, "remote file", "remote\n");
    fixture.commit(&source);
    let output = fast_forward(&fixture.root.join("workspace"), "ff");
    assert_eq!(output.status.code(), Some(1));
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("diverged"));
    assert!(errors.contains("remote \"bad\""));
    assert_eq!(head(&fixture, &diverged), diverged_before);
    assert_eq!(head(&fixture, &broken), broken_before);
    assert_eq!(
        fixture.git(&broken, &["rev-parse", "origin/main"]),
        head(&fixture, &source)
    );
}

#[test]
fn skips_detached_unborn_and_untracked_branches() {
    let fixture = Fixture::new();
    fixture.repo("workspace/unborn", false);
    let standalone = fixture.repo("workspace/standalone", true);
    let standalone_before = head(&fixture, &standalone);
    let detached = fixture.repo("workspace/detached", true);
    fixture.git(&detached, &["checkout", "--detach"]);
    let before = head(&fixture, &detached);
    let output = fast_forward(&fixture.root.join("workspace"), "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("· unborn: skipped: no commits"));
    assert!(text.contains("· standalone: skipped: no upstream"));
    assert!(text.contains("· detached: skipped: detached HEAD"));
    assert_eq!(head(&fixture, &detached), before);
    assert_eq!(head(&fixture, &standalone), standalone_before);
}

#[test]
fn protects_ignored_files_and_rejects_operations_in_progress() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    write(&source, ".gitignore", "protected\n");
    fixture.commit(&source);
    let local = tracking_repo(&fixture, "workspace/local", &source);
    let before = head(&fixture, &local);
    let index_before = std::fs::read(local.join(".git/index")).expect("index");
    write(&local, "protected", "local ignored content\n");
    write(&source, "protected", "remote content\n");
    fixture.git(&source, &["add", "--force", "protected"]);
    fixture.commit(&source);
    let output = fast_forward(&local, "ff");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(head(&fixture, &local), before);
    assert_eq!(
        std::fs::read(local.join(".git/index")).expect("index"),
        index_before
    );
    assert_eq!(
        std::fs::read_to_string(local.join("protected")).expect("file"),
        "local ignored content\n"
    );
    write(&local, ".git/MERGE_HEAD", &format!("{before}\n"));
    let output = fast_forward(&local, "ff");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("operation in progress"));
    assert_eq!(head(&fixture, &local), before);
}

#[test]
fn locks_prevent_checkout_and_are_released_after_failure() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let local = tracking_repo(&fixture, "workspace/local", &source);
    let before = head(&fixture, &local);
    let index_before = std::fs::read(local.join(".git/index")).expect("index");
    write(&source, "tracked file", "remote edits\n");
    fixture.commit(&source);
    for lock in [
        ".git/refs/heads/main.lock",
        ".git/HEAD.lock",
        ".git/index.lock",
    ] {
        write(&local, lock, "held by another operation\n");
        let output = fast_forward(&local, "ff");
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(head(&fixture, &local), before);
        assert_eq!(
            std::fs::read(local.join(".git/index")).expect("index"),
            index_before
        );
        assert_eq!(
            std::fs::read_to_string(local.join("tracked file")).expect("file"),
            "base\n"
        );
        assert_eq!(
            std::fs::read_to_string(local.join(lock)).expect("lock"),
            "held by another operation\n"
        );
        std::fs::remove_file(local.join(lock)).expect("remove fixture lock");
    }
    let output = fast_forward(&local, "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(head(&fixture, &local), head(&fixture, &source));
}

#[test]
fn local_upstream_updates_files_index_and_reflog() {
    let fixture = Fixture::new();
    let local = fixture.repo("local", true);
    fixture.git(&local, &["checkout", "-b", "upstream"]);
    fixture.git(&local, &["rm", "tracked file"]);
    write(&local, "nested/new file", "new content\n");
    fixture.commit(&local);
    let target = head(&fixture, &local);
    fixture.git(&local, &["checkout", "main"]);
    fixture.git(&local, &["branch", "--set-upstream-to=upstream"]);
    let output = fast_forward(&local, "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(head(&fixture, &local), target);
    assert!(!local.join("tracked file").exists());
    assert_eq!(
        std::fs::read_to_string(local.join("nested/new file")).expect("file"),
        "new content\n"
    );
    assert!(fixture.git(&local, &["status", "--porcelain"]).is_empty());
    assert_eq!(
        fixture.git(&local, &["reflog", "-1", "--format=%H", "main"]),
        target
    );
}

#[test]
fn skips_gone_upstream_and_handles_empty_workspace() {
    let fixture = Fixture::new();
    let output = fast_forward(&fixture.root, "ff");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"No repositories found.\n");
    let local = fixture.repo("local", true);
    fixture.git(&local, &["config", "branch.main.remote", "."]);
    fixture.git(
        &local,
        &["config", "branch.main.merge", "refs/heads/missing"],
    );
    let before = head(&fixture, &local);
    let output = fast_forward(&local, "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\xc2\xb7 .: skipped: upstream gone\n");
    assert_eq!(head(&fixture, &local), before);
}

#[test]
fn leaves_submodules_pinned_unless_requested() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let parent = fixture.repo("workspace/parent", true);
    let child = fixture.add_submodule(&parent, &source, "child");
    fixture.commit(&parent);
    let pinned = head(&fixture, &child);
    write(&source, "tracked file", "remote edits\n");
    fixture.commit(&source);

    let output = fast_forward(&fixture.root.join("workspace"), "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("parent: skipped: no upstream"));
    assert!(!text.contains("child"), "{text}");
    assert_eq!(head(&fixture, &child), pinned);

    // An explicitly selected root is updated even when it is a submodule.
    let output = fast_forward(&child, "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"\xe2\x9c\x93 .: fast-forwarded\n");
    assert_eq!(head(&fixture, &child), head(&fixture, &source));
    write(&source, "tracked file", "more remote edits\n");
    fixture.commit(&source);

    let output = command()
        .args(["ff", "--submodules", "--root"])
        .arg(fixture.root.join("workspace"))
        .output()
        .expect("fast-forward command should execute");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("parent/child: fast-forwarded"));
    assert_eq!(head(&fixture, &child), head(&fixture, &source));
}

#[test]
fn prefers_base_checkout_and_supports_worktree_only_roots() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let base = tracking_repo(&fixture, "workspace/z-base", &source);
    let linked = fixture.root.join("workspace/a-linked");
    fixture.git(
        &base,
        &[
            "worktree",
            "add",
            "-b",
            "topic",
            linked.to_str().expect("UTF-8 path"),
        ],
    );
    fixture.git(&linked, &["branch", "--set-upstream-to=origin/main"]);
    let before = head(&fixture, &linked);
    write(&source, "new file", "remote\n");
    fixture.commit(&source);
    let output = fast_forward(&fixture.root.join("workspace"), "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "✓ z-base: fast-forwarded"
    );
    assert_eq!(head(&fixture, &base), head(&fixture, &source));
    assert_eq!(head(&fixture, &linked), before);
    let output = fast_forward(&linked, "ff");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(head(&fixture, &linked), head(&fixture, &source));
}
