//! AC1: Given fixture shims for attest/roster/converge/arbiter/tether on PATH
//! returning known JSON, `corpus-introspect --json` emits a WholeSelf record
//! whose nodes, leases, and converged fields correctly reflect the fixtures
//! (golden-compared).

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
    p.pop(); // remove test binary name
    // Go up from deps/ to the build dir
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("corpus-introspect")
}

#[test]
fn ac1_whole_self_golden() {
    let dir = TempDir::new().expect("tempdir");
    let shim_dir = dir.path();

    make_shim(
        shim_dir,
        "corpus-attest",
        r#"{"this_node":"laptop","attested_nodes":["laptop","server"]}"#,
    );
    make_shim(
        shim_dir,
        "muster",
        r#"{"nodes":[{"node":"laptop","sessions":2},{"node":"server","sessions":1}]}"#,
    );
    make_shim(
        shim_dir,
        "corpus-converge",
        r#"{"converged":true,"nodes":[{"node":"laptop","lag":0},{"node":"server","lag":0}]}"#,
    );
    make_shim(
        shim_dir,
        "corpus-arbiter",
        r#"{"leases":[{"key":"lock/build","holder":"laptop","expires":"2026-06-17T01:00:00Z"}]}"#,
    );
    make_shim(
        shim_dir,
        "wm-tether",
        r#"{"links":[{"node":"server","up":true,"rtt_ms":8.2}]}"#,
    );

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shim_dir.display(), old_path);

    let output = std::process::Command::new(bin_path())
        .arg("--json")
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    // nodes array has 2 entries
    let nodes = v["nodes"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 2, "expected 2 nodes: {stdout}");

    // converged == true
    assert_eq!(v["converged"], serde_json::json!(true), "converged field");

    // leases has 1 entry with key lock/build
    let leases = v["leases"].as_array().expect("leases array");
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0]["key"], "lock/build");

    // generated_ts is present
    assert!(v["generated_ts"].is_string(), "generated_ts must be string");

    // laptop node is attested
    let laptop = nodes.iter().find(|n| n["node"] == "laptop").expect("laptop node");
    assert_eq!(laptop["attested"], true, "laptop must be attested");

    // server node is attested
    let server = nodes.iter().find(|n| n["node"] == "server").expect("server node");
    assert_eq!(server["attested"], true, "server must be attested");
}
