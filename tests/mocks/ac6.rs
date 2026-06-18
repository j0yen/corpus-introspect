//! Mock for AC6 (deferred — live two-node fleet with real NATS).
//!
//! AC6 requires a real two-node fleet with live link RTT and real session counts.
//! This mock exercises the same public API surface the real test would use,
//! against a documented in-crate fake (shim scripts returning fixture JSON),
//! and asserts the same invariant the AC's English text declares:
//! both nodes are listed with link RTT and session counts.
//!
//! The real test (tests/acceptance_ac6.rs, not yet written) would run against
//! a live NATS bus and real corpus components. This mock proves the call
//! sequence + signature + invariant at the type level.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::TempDir;

fn make_shim(dir: &std::path::Path, name: &str, json_output: &str) {
    let path = dir.join(name);
    let content = format!(
        "#!/bin/sh\ncat <<'__EOF__'\n{}\n__EOF__\n",
        json_output
    );
    fs::write(&path, content).expect("write shim");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .expect("chmod shim");
}

fn bin_path() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("corpus-introspect")
}

/// Mock: verifies that when two nodes appear in the fixture data, both are
/// listed in the JSON output with rtt_ms and session counts present.
/// This is the same invariant AC6 would assert on a live fleet.
#[test]
fn ac6_mock_two_node_fleet_shape() {
    let dir = TempDir::new().expect("tempdir");
    let shim_dir = dir.path();

    // Two-node fixture simulating a real fleet.
    make_shim(
        shim_dir,
        "corpus-attest",
        r#"{"this_node":"node-a","attested_nodes":["node-a","node-b"]}"#,
    );
    make_shim(
        shim_dir,
        "muster",
        r#"{"nodes":[{"node":"node-a","sessions":5},{"node":"node-b","sessions":3}]}"#,
    );
    make_shim(
        shim_dir,
        "corpus-converge",
        r#"{"converged":true,"nodes":[{"node":"node-a","lag":0},{"node":"node-b","lag":0}]}"#,
    );
    make_shim(shim_dir, "corpus-arbiter", r#"{"leases":[]}"#);
    // Fixture with live-style RTT values.
    make_shim(
        shim_dir,
        "wm-tether",
        r#"{"links":[{"node":"node-b","up":true,"rtt_ms":14.7}]}"#,
    );

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shim_dir.display(), old_path);

    let output = std::process::Command::new(bin_path())
        .arg("--json")
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(output.status.success(), "exit 0");

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let nodes = v["nodes"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 2, "two-node fleet: expect 2 nodes");

    // Both nodes present by name.
    let names: Vec<&str> = nodes
        .iter()
        .filter_map(|n| n["node"].as_str())
        .collect();
    assert!(names.contains(&"node-a"), "node-a present");
    assert!(names.contains(&"node-b"), "node-b present");

    // node-b has a link with rtt_ms.
    let node_b = nodes.iter().find(|n| n["node"] == "node-b").expect("node-b");
    let rtt = node_b["link"]["rtt_ms"].as_f64().expect("rtt_ms present");
    assert!(rtt > 0.0, "rtt_ms should be positive");

    // Both nodes have session counts.
    for node in nodes {
        assert!(
            node["sessions"].is_number(),
            "node {} should have session count",
            node["node"]
        );
    }
}
