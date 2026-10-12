mod support;
use support::*;

#[test]
fn root_precedence_help_and_exit_codes() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repository with spaces", true);
    let missing = command().output().expect("repos command should execute");
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        String::from_utf8(missing.stderr)
            .expect("test output should be valid UTF-8")
            .contains("provide --root")
    );
    let environment = command()
        .env("REPOS_ROOT", &repo)
        .output()
        .expect("repos command should execute");
    assert!(environment.status.success());
    assert_eq!(
        row(
            &String::from_utf8(environment.stdout).expect("test output should be valid UTF-8"),
            "."
        )
        .state,
        "clean"
    );
    let explicit = command()
        .env("REPOS_ROOT", fixture.root.join("absent"))
        .arg("--root")
        .arg(&repo)
        .output()
        .expect("repos command should execute");
    assert!(explicit.status.success());
    let status = command()
        .arg("status")
        .arg("--root")
        .arg(&repo)
        .output()
        .expect("repos command should execute");
    assert_eq!(status.stdout, explicit.stdout);
    for args in [vec!["--help"], vec!["-h"], vec!["status", "--help"]] {
        let is_status = args.first() == Some(&"status");
        let help = command()
            .args(args)
            .output()
            .expect("repos command should execute");
        assert!(help.status.success());
        assert!(help.stderr.is_empty());
        let text = String::from_utf8(help.stdout).expect("test output should be valid UTF-8");
        if is_status {
            assert!(text.contains("Usage: repos status [OPTIONS]"));
        } else {
            assert!(text.contains("Usage: repos [OPTIONS]"));
            assert!(text.contains("repos <COMMAND>"));
            assert!(text.contains("Summarize local Git repositories"));
        }
        assert!(text.contains("--max-depth"));
        assert!(text.contains("S staged"));
    }
    let invalid = command()
        .args(["--max-depth", "-2"])
        .output()
        .expect("repos command should execute");
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    assert!(!invalid.stderr.is_empty());
    assert_eq!(
        run(&fixture.root.join("missing"), &[]).status.code(),
        Some(1)
    );
    assert_eq!(run(&repo.join("tracked file"), &[]).status.code(), Some(1));
    let empty = summary(&fixture.root.join("repository with spaces"), &[]);
    assert!(empty.contains("clean"));
    assert_eq!(
        summary(&repo, &["--dirty"]),
        "No repositories need attention.\n"
    );
}

#[test]
fn empty_and_redirected_output() {
    let fixture = Fixture::new();
    assert_eq!(summary(&fixture.root, &[]), "No repositories found.\n");
    fixture.repo("repo", true);
    let output = command()
        .arg("--root")
        .arg(&fixture.root)
        .env("TERM", "xterm")
        .env_remove("NO_COLOR")
        .output()
        .expect("repos command should execute");
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    assert!(output.stderr.is_empty());
}

#[test]
fn inspection_failure_keeps_other_repositories_visible() {
    let fixture = Fixture::new();
    fixture.repo("good", true);
    write(&fixture.root, "broken/.git", "gitdir: missing\n");
    let output = run(&fixture.root, &[]);
    assert_eq!(output.status.code(), Some(1));
    let table = String::from_utf8(output.stdout).expect("test output should be valid UTF-8");
    assert_eq!(row(&table, "good").state, "clean");
    assert_eq!(row(&table, "broken").state, "error");
    assert!(!output.stderr.is_empty());
}
