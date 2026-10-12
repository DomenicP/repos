mod support;
use support::*;

#[test]
fn dirty_submodules_override_hiding_config_and_count_staged_gitlinks() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let parent = fixture.repo("parent", true);
    let child = fixture.add_submodule(&parent, &source, "child with spaces");
    fixture.git(
        &parent,
        &["config", "submodule.child with spaces.ignore", "all"],
    );
    assert_eq!(row(&summary(&parent, &[]), ".").changes, "S:2");
    fixture.commit(&parent);
    write(&child, "tracked file", "modified\n");
    write(&child, "untracked", "new\n");
    let child_index = std::path::PathBuf::from(fixture.git(
        &child,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
    ));
    let protected = [
        parent.join(".git/index"),
        parent.join(".git/config"),
        child_index,
    ];
    let before: Vec<_> = protected
        .iter()
        .map(|p| std::fs::read(p).expect("fixture file should be readable"))
        .collect();
    let collapsed = summary(&parent, &[]);
    assert_eq!(rows(&collapsed).len(), 1);
    assert_eq!(row(&collapsed, ".").changes, "M:1 subM:1 sub?:1");
    let expanded = summary(&parent, &["--submodules"]);
    assert_eq!(rows(&expanded).len(), 2);
    assert_eq!(row(&expanded, "child with spaces").changes, "M:1 ?:1");
    assert!(
        expanded
            .lines()
            .any(|line| line.starts_with("  child with spaces"))
    );
    for (path, bytes) in protected.iter().zip(before) {
        assert_eq!(
            std::fs::read(path).expect("fixture file should be readable"),
            bytes,
            "inspection wrote {path:?}"
        );
    }
    fixture.commit(&child);
    assert_eq!(row(&summary(&parent, &[]), ".").changes, "M:1 subC:1");
    fixture.git(&parent, &["add", "child with spaces"]);
    assert_eq!(row(&summary(&parent, &[]), ".").changes, "S:1");
    fixture.commit(&parent);
    fixture.git(&parent, &["mv", "child with spaces", "renamed child"]);
    assert_eq!(row(&summary(&parent, &[]), ".").changes, "S:2");
    fixture.commit(&parent);
    fixture.git(&parent, &["rm", "--force", "renamed child"]);
    assert_eq!(row(&summary(&parent, &[]), ".").changes, "S:2");
}

#[test]
fn uninitialized_submodule_has_a_child_row_without_being_initialized() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let parent = fixture.repo("parent", true);
    fixture.add_submodule(&parent, &source, "child");
    fixture.commit(&parent);
    fixture.git(&parent, &["submodule", "deinit", "--force", "child"]);
    let output = summary(&parent, &["--dirty", "--submodules"]);
    assert_eq!(row(&output, ".").changes, "subU:1");
    assert_eq!(row(&output, "child").state, "uninitialized");
    assert!(!parent.join("child/.git").exists());
}

#[test]
fn nested_untracked_content_and_uninitialized_descendants() {
    let fixture = Fixture::new();
    let source = fixture.repo("source", true);
    let middle = fixture.repo("middle", true);
    fixture.add_submodule(&middle, &source, "inner");
    fixture.commit(&middle);
    let outer = fixture.repo("outer", true);
    let child = fixture.add_submodule(&outer, &middle, "middle");
    fixture.git(
        &outer,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
            "--recursive",
        ],
    );
    fixture.commit(&outer);
    let inner = child.join("inner");
    write(&inner, "untracked", "new\n");
    let output = summary(&outer, &["--submodules"]);
    assert_eq!(row(&output, ".").changes, "M:1 sub?:1");
    assert_eq!(row(&output, "middle").changes, "M:1 sub?:1");
    assert_eq!(row(&output, "middle/inner").changes, "?:1");
    write(&inner, "tracked file", "modified\n");
    let output = summary(&outer, &["--submodules"]);
    assert_eq!(row(&output, ".").changes, "M:1 subM:1 sub?:1");
    assert_eq!(row(&output, "middle").changes, "M:1 subM:1 sub?:1");
    fixture.git(&child, &["submodule", "deinit", "--force", "inner"]);
    let limited = summary(&outer, &["--max-depth", "0", "--dirty", "--submodules"]);
    assert_eq!(rows(&limited).len(), 1);
    assert_eq!(row(&limited, ".").changes, "subU:1");
    let expanded = summary(&outer, &["--submodules"]);
    assert_eq!(row(&expanded, ".").changes, "subU:1");
    assert_eq!(row(&expanded, "middle").changes, "subU:1");
    assert_eq!(row(&expanded, "middle/inner").state, "uninitialized");
}
