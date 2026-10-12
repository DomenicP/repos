use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

#[test]
#[ignore = "build the release binary and set REPOS_BENCH_ROOT"]
fn compiled_cli_latency() {
    let root = std::env::var_os("REPOS_BENCH_ROOT").expect("set REPOS_BENCH_ROOT");
    let binary = std::env::var_os("REPOS_BENCH_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/repos"));
    let mut samples = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        let output = Command::new(&binary)
            .arg("--root")
            .arg(&root)
            .stdout(Stdio::null())
            .output()
            .expect("build the release binary or set REPOS_BENCH_BINARY");
        samples.push(start.elapsed());
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    samples.sort();
    let median = (samples[4] + samples[5]) / 2;
    println!(
        "10 compiled CLI runs: median {median:.2?}, min {:.2?}, max {:.2?}",
        samples[0], samples[9]
    );
}
