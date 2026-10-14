mod support;
use support::*;

fn fetch(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    command()
        .arg("fetch")
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .expect("fetch command should execute")
}

#[test]
fn fetches_every_remote_without_changing_head_or_worktree() {
    let fixture = Fixture::new();
    let first = fixture.repo("sources/first", true);
    let second = fixture.repo("sources/second", true);
    let local = fixture.repo("workspace/local", true);
    for (name, source) in [("origin", &first), ("other", &second)] {
        fixture.git(
            &local,
            &[
                "remote",
                "add",
                name,
                source.to_str().expect("fixture path is UTF-8"),
            ],
        );
    }
    let head = fixture.git(&local, &["rev-parse", "HEAD"]);
    write(&local, "tracked file", "local edits\n");
    let before = fixture.git(&local, &["status", "--porcelain"]);
    let output = fetch(&fixture.root.join("workspace"), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("local: fetched 2 remotes"));
    assert!(
        output.stderr.is_empty(),
        "redirected fetch should only print final results"
    );
    assert!(
        !output.stdout.contains(&0x1b),
        "redirected output should have no terminal escapes"
    );
    for (name, source) in [("origin", &first), ("other", &second)] {
        assert_eq!(
            fixture.git(&local, &["rev-parse", &format!("refs/remotes/{name}/main")]),
            fixture.git(source, &["rev-parse", "HEAD"])
        );
    }
    write(&first, "new file", "new commit\n");
    fixture.commit(&first);
    assert!(fetch(&fixture.root.join("workspace"), &[]).status.success());
    assert_eq!(
        fixture.git(&local, &["rev-parse", "origin/main"]),
        fixture.git(&first, &["rev-parse", "HEAD"])
    );
    assert_eq!(fixture.git(&local, &["rev-parse", "HEAD"]), head);
    assert_eq!(fixture.git(&local, &["status", "--porcelain"]), before);
}

#[test]
fn failures_do_not_stop_other_remotes_or_repositories() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let broken = fixture.repo("workspace/broken", true);
    let good = fixture.repo("workspace/good", true);
    for repo in [&broken, &good] {
        fixture.git(
            repo,
            &[
                "remote",
                "add",
                "origin",
                source.to_str().expect("fixture path is UTF-8"),
            ],
        );
    }
    fixture.git(
        &broken,
        &[
            "remote",
            "add",
            "bad",
            fixture
                .root
                .join("absent")
                .to_str()
                .expect("fixture path is UTF-8"),
        ],
    );
    let output = fetch(&fixture.root.join("workspace"), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("remote \"bad\""));
    assert!(String::from_utf8_lossy(&output.stdout).contains("good: fetched 1 remotes"));
    for repo in [&broken, &good] {
        assert_eq!(
            fixture.git(repo, &["rev-parse", "origin/main"]),
            fixture.git(&source, &["rev-parse", "HEAD"])
        );
    }
}

#[test]
fn respects_depth_and_handles_repositories_without_remotes() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let local = fixture.repo("workspace/nested/local", true);
    fixture.repo("workspace/standalone", true);
    fixture.git(
        &local,
        &[
            "remote",
            "add",
            "origin",
            source.to_str().expect("fixture path is UTF-8"),
        ],
    );
    let shallow = fetch(&fixture.root.join("workspace"), &["--max-depth", "1"]);
    assert!(shallow.status.success());
    assert!(String::from_utf8_lossy(&shallow.stdout).contains("standalone: no remotes"));
    assert!(
        !fixture
            .git_output(
                &local,
                &["show-ref", "--verify", "refs/remotes/origin/main"]
            )
            .status
            .success()
    );
    let deep = command()
        .arg("fetch")
        .env("REPOS_ROOT", fixture.root.join("workspace"))
        .args(["--max-depth", "-1"])
        .output()
        .expect("fetch command should execute");
    assert!(deep.status.success());
    assert!(
        fixture
            .git_output(
                &local,
                &["show-ref", "--verify", "refs/remotes/origin/main"]
            )
            .status
            .success()
    );
}
