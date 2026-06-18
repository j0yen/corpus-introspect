//! Proptest invariants for corpus-introspect.
//! READ-ONLY: the edit-agent must not modify this file.
//!
//! These check structural invariants of the data model, not specific
//! output values (which are tested in acceptance_*.rs).

// No proptest dep currently — structural-only checks using std.
// Add proptest crate if property-based testing is added in a future iter.

/// The WholeSelf::collect() call must always produce at least one node
/// (the lone-laptop invariant from AC4).
#[test]
fn invariant_at_least_one_node() {
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

    let dir = TempDir::new().expect("tempdir");
    let new_path = format!("{}:/usr/bin:/bin", dir.path().display());

    let output = std::process::Command::new(bin_path())
        .arg("--json")
        .env("PATH", &new_path)
        .output()
        .expect("run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let nodes = v["nodes"].as_array().expect("nodes array");
    assert!(!nodes.is_empty(), "invariant: at least one node always");
}

/// The generated_ts field must always be a non-empty string.
#[test]
fn invariant_generated_ts_always_present() {
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

    let dir = TempDir::new().expect("tempdir");
    let new_path = format!("{}:/usr/bin:/bin", dir.path().display());

    let output = std::process::Command::new(bin_path())
        .arg("--json")
        .env("PATH", &new_path)
        .output()
        .expect("run");

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let ts = v["generated_ts"].as_str().expect("generated_ts string");
    assert!(!ts.is_empty(), "generated_ts must be non-empty");
}
