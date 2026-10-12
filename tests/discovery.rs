mod support;
use support::*;

#[test]
fn nested_hidden_repositories_and_depth_limits() {
    let fixture = Fixture::new();
    fixture.repo("z", true);
    fixture.repo("a", true);
    fixture.repo("a/nested", true);
    fixture.repo(".hidden/repo", true);
    fixture.repo("one/two/three", true);
    fixture.repo("one/two/three/four", true);
    let paths = |args: &[&str]| {
        rows(&summary(&fixture.root, args))
            .into_iter()
            .map(|r| r.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        paths(&[]),
        [".hidden/repo", "a", "a/nested", "one/two/three", "z"]
    );
    assert_eq!(paths(&["--max-depth", "1"]), ["a", "z"]);
    assert_eq!(paths(&["--max-depth", "0"]), Vec::<String>::new());
    assert_eq!(
        paths(&["--max-depth", "-1"]),
        [
            ".hidden/repo",
            "a",
            "a/nested",
            "one/two/three",
            "one/two/three/four",
            "z"
        ]
    );
}

#[test]
fn linked_worktrees_are_discovered() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    fixture.git(
        &source,
        &[
            "worktree",
            "add",
            "--detach",
            fixture
                .root
                .join("linked")
                .to_str()
                .expect("fixture path should be valid UTF-8"),
        ],
    );
    let output = summary(&fixture.root, &[]);
    assert_eq!(row(&output, "linked").state, "detached,clean");
    assert_eq!(row(&output, "source").state, "clean");
}

#[cfg(unix)]
#[test]
fn child_symlinks_are_skipped_and_explicit_root_symlink_is_resolved() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let link = fixture.root.join("link");
    std::os::unix::fs::symlink(&source, &link).expect("fixture symlink should be created");
    assert_eq!(rows(&summary(&fixture.root, &[])).len(), 1);
    assert_eq!(row(&summary(&link, &[]), ".").state, "clean");
}

// This macOS filesystem rejects the fixture name with EILSEQ. Linux permits
// byte filenames; rendering's byte-escape unit test also runs on macOS.
#[cfg(target_os = "linux")]
#[test]
fn non_utf8_paths_are_opened_and_displayed_without_replacement() {
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let path = fixture
        .root
        .join(std::ffi::OsString::from_vec(b"repo\xff".to_vec()));
    std::fs::rename(&source, &path).expect("fixture repository should be renamed");
    assert_eq!(
        row(&summary(&fixture.root, &[]), "\"repo\\xff\"").state,
        "clean"
    );
    assert_eq!(row(&summary(&path, &[]), ".").state, "clean");
    let mut inline = std::ffi::OsString::from("--root=");
    inline.push(&path);
    assert!(
        command()
            .arg(inline)
            .output()
            .expect("repos command should execute")
            .status
            .success()
    );
}

#[cfg(unix)]
#[test]
fn unreadable_directory_does_not_hide_other_repositories() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.repo("good", true);
    let blocked = fixture.root.join("blocked");
    std::fs::create_dir(&blocked).expect("fixture directory should be created");
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o0))
        .expect("fixture permissions should be updated");
    let output = run(&fixture.root, &[]);
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o755))
        .expect("fixture permissions should be updated");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        row(
            &String::from_utf8(output.stdout).expect("test output should be valid UTF-8"),
            "good"
        )
        .state,
        "clean"
    );
    assert!(
        String::from_utf8(output.stderr)
            .expect("test output should be valid UTF-8")
            .contains("discover")
    );
}
