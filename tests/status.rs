mod support;
use support::*;

#[test]
fn no_commits_changes_renames_and_detached_head() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repository with spaces", false);
    assert!(!repo.join(".git/index").exists());
    let initial = row(&summary(&repo, &[]), ".");
    assert!(!repo.join(".git/index").exists());
    assert_eq!(initial.branch, "main");
    assert_eq!(initial.state, "no-commits,clean");
    write(&repo, "tracked file", "base\n");
    fixture.commit(&repo);
    fixture.git(&repo, &["config", "status.showUntrackedFiles", "no"]);
    write(&repo, "tracked file", "staged\n");
    fixture.git(&repo, &["add", "tracked file"]);
    write(&repo, "tracked file", "modified\n");
    write(&repo, "new\tfile\nwith spaces", "untracked\n");
    let changed = row(&summary(&repo, &[]), ".");
    assert_eq!(changed.state, "dirty");
    assert_eq!(changed.changes, "S:1 M:1 ?:1");
    fixture.commit(&repo);
    fixture.git(&repo, &["mv", "tracked file", "renamed\nfile with spaces"]);
    assert_eq!(row(&summary(&repo, &[]), ".").changes, "S:1");
    fixture.commit(&repo);
    fixture.git(&repo, &["checkout", "--detach"]);
    assert_eq!(row(&summary(&repo, &[]), ".").state, "detached,clean");
    assert_eq!(
        summary(&repo, &["--dirty"]),
        "No repositories need attention.\n"
    );
}

#[test]
fn upstream_states_and_conflicts() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repo", true);
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "-");
    fixture.git(&repo, &["branch", "tracking"]);
    fixture.git(&repo, &["branch", "--set-upstream-to=tracking", "main"]);
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "tracking =");
    write(&repo, "tracked file", "main change\n");
    fixture.commit(&repo);
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "tracking +1");
    assert_eq!(
        summary(&repo, &["--dirty"]),
        "No repositories need attention.\n"
    );
    fixture.git(&repo, &["checkout", "tracking"]);
    fixture.git(&repo, &["branch", "--set-upstream-to=main", "tracking"]);
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "main -1");
    write(&repo, "tracked file", "tracking change\n");
    fixture.commit(&repo);
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "main +1/-1");
    assert!(
        !fixture
            .git_output(&repo, &["merge", "main"])
            .status
            .success()
    );
    let conflict = row(&summary(&repo, &["--dirty"]), ".");
    assert_eq!(conflict.state, "conflict");
    assert_eq!(conflict.changes, "!:1");
}

#[test]
fn missing_remote_tracking_ref_is_unknown() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repo", true);
    fixture.git(
        &repo,
        &[
            "remote",
            "add",
            "origin",
            fixture
                .root
                .join("unused")
                .to_str()
                .expect("fixture path should be valid UTF-8"),
        ],
    );
    fixture.git(&repo, &["config", "branch.main.remote", "origin"]);
    fixture.git(&repo, &["config", "branch.main.merge", "refs/heads/main"]);
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "origin ?");
    fixture.git(
        &repo,
        &[
            "symbolic-ref",
            "refs/remotes/origin/main",
            "refs/remotes/origin/missing",
        ],
    );
    assert_eq!(row(&summary(&repo, &[]), ".").upstream, "origin ?");
}

#[test]
fn inspection_never_persists_index_refreshes() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repo", true);
    let before = std::fs::read(repo.join(".git/index")).expect("fixture file should be readable");
    // Different size guarantees stale stat metadata without sleeping or relying
    // on the filesystem's timestamp precision.
    write(
        &repo,
        "tracked file",
        "changed content, stale index metadata\n",
    );
    for _ in 0..3 {
        assert_eq!(row(&summary(&repo, &[]), ".").changes, "M:1");
    }
    assert_eq!(
        before,
        std::fs::read(repo.join(".git/index")).expect("fixture file should be readable")
    );
}

#[test]
fn deleted_paths_and_ignored_files() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repo", true);
    write(&repo, ".gitignore", "*.log\n");
    write(&repo, "removed file", "base\n");
    fixture.commit(&repo);
    fixture.git(&repo, &["rm", "removed file"]);
    std::fs::remove_file(repo.join("tracked file"))
        .expect("tracked fixture file should be removed");
    write(&repo, "ignored.log", "ignored\n");
    write(&repo, "new file", "untracked\n");
    assert_eq!(row(&summary(&repo, &[]), ".").changes, "S:1 M:1 ?:1");
}
