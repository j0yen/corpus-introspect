//! AC4: When NO corpus components are installed/configured, `corpus introspect`
//! reports a single-node self (just this box, unattested, no fleet) honestly,
//! rather than erroring.

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
fn ac4_lone_laptop_case() {
    // Use an empty temp dir as PATH — no corpus CLIs available at all.
    let dir = TempDir::new().expect("tempdir");
    let empty_shim_dir = dir.path();
    // Minimal PATH: only system bins needed to run the binary itself.
    let new_path = format!("{}:/usr/bin:/bin", empty_shim_dir.display());

    let output = std::process::Command::new(bin_path())
        .arg("--json")
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(
        output.status.success(),
        "exit 0 in lone-laptop case\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    // At least one node (this box).
    let nodes = v["nodes"].as_array().expect("nodes array");
    assert!(!nodes.is_empty(), "must report at least one node");

    // That node is NOT attested (no corpus-attest available).
    let first = &nodes[0];
    assert_eq!(first["attested"], false, "lone node must be unattested");

    // All facets should be degraded.
    let facet_status = v["facet_status"].as_object().expect("facet_status object");
    for (facet, status) in facet_status {
        assert_ne!(
            status.as_str(),
            Some("ok"),
            "facet {facet} should not be ok in lone-laptop mode"
        );
    }
}

#[test]
fn ac4_lone_laptop_text_view() {
    let dir = TempDir::new().expect("tempdir");
    let empty_shim_dir = dir.path();
    let new_path = format!("{}:/usr/bin:/bin", empty_shim_dir.display());

    let output = std::process::Command::new(bin_path())
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(output.status.success(), "exit 0 in lone-laptop text mode");

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    // Should mention "unattested" — the honest lone-laptop label.
    assert!(
        stdout.contains("unattested"),
        "text view must say 'unattested' for lone laptop\n{stdout}"
    );
}
