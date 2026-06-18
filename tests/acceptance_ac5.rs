//! AC5: `--format selfreview` emits a parseable block containing the node
//! count, converged verdict, and held-lease count, suitable for the
//! self-review playbook (structural assert).

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

#[test]
fn ac5_selfreview_format_structural() {
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
        r#"{"leases":[{"key":"lock/build","holder":"laptop"}]}"#,
    );
    make_shim(shim_dir, "wm-tether", r#"{"links":[]}"#);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shim_dir.display(), old_path);

    let output = std::process::Command::new(bin_path())
        .args(["--format", "selfreview"])
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect --format selfreview");

    assert!(output.status.success(), "exit 0 for selfreview format");

    let stdout = String::from_utf8(output.stdout).expect("utf8");

    // Must contain node_count
    assert!(
        stdout.contains("node_count:"),
        "selfreview block must contain node_count:\n{stdout}"
    );

    // node_count must be 2
    let node_count_line = stdout
        .lines()
        .find(|l| l.starts_with("node_count:"))
        .expect("node_count line");
    let count_val = node_count_line
        .split(':')
        .nth(1)
        .map(str::trim)
        .expect("count value");
    assert_eq!(count_val, "2", "node_count should be 2");

    // Must contain converged verdict
    assert!(
        stdout.contains("converged:"),
        "selfreview block must contain converged:\n{stdout}"
    );
    let converged_line = stdout
        .lines()
        .find(|l| l.starts_with("converged:"))
        .expect("converged line");
    assert!(
        converged_line.contains("yes") || converged_line.contains("no"),
        "converged must be 'yes' or 'no': {converged_line}"
    );

    // Must contain lease_count
    assert!(
        stdout.contains("lease_count:"),
        "selfreview block must contain lease_count:\n{stdout}"
    );
    let lease_line = stdout
        .lines()
        .find(|l| l.starts_with("lease_count:"))
        .expect("lease_count line");
    let lease_val = lease_line
        .split(':')
        .nth(1)
        .map(str::trim)
        .expect("lease count value");
    assert_eq!(lease_val, "1", "lease_count should be 1");
}
