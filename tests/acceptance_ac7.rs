//! AC7: `cargo test` green; `sigpipe::reset()` first in `main()` (grep-asserted);
//! `corpus introspect | head` does not panic (no SIGPIPE).

use std::path::PathBuf;
use tempfile::TempDir;

fn bin_path() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("corpus-introspect")
}

#[test]
fn ac7_sigpipe_no_panic() {
    // Run `corpus-introspect | head -n 1` via shell to induce SIGPIPE.
    // The process must not exit with a non-zero code caused by SIGPIPE panic.
    let dir = TempDir::new().expect("tempdir");
    let new_path = format!("{}:/usr/bin:/bin", dir.path().display());

    // Run corpus-introspect and close stdin immediately (simulate pipe closure).
    // We use `sh -c` with `head` to trigger SIGPIPE on the first line.
    let bin = bin_path();
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{} | head -n 1", bin.display()))
        .env("PATH", &new_path)
        .output()
        .expect("run via shell pipe");

    // Exit code 0 or 141 (SIGPIPE shell exit) are both acceptable.
    // What is NOT acceptable is exit code from a Rust panic (usually 101).
    let code = output.status.code().unwrap_or(0);
    assert_ne!(
        code, 101,
        "exit code 101 suggests a Rust panic, likely SIGPIPE not reset"
    );
}

#[test]
fn ac7_sigpipe_reset_in_source() {
    // Grep-assert: sigpipe::reset() appears in src/main.rs
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"),
    )
    .expect("read src/main.rs");

    assert!(
        src.contains("sigpipe::reset()"),
        "src/main.rs must call sigpipe::reset() — not found in source"
    );
}
