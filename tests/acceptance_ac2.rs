//! AC2: The text view renders, for the multi-node fixture, a self-portrait
//! naming each node, its attested+link status, its session count, and the
//! converged/lagging verdict on a bounded number of lines (snapshot-tested).

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
fn ac2_text_view_multi_node() {
    let dir = TempDir::new().expect("tempdir");
    let shim_dir = dir.path();

    make_shim(
        shim_dir,
        "corpus-attest",
        r#"{"this_node":"alpha","attested_nodes":["alpha","beta"]}"#,
    );
    make_shim(
        shim_dir,
        "muster",
        r#"{"nodes":[{"node":"alpha","sessions":3},{"node":"beta","sessions":0}]}"#,
    );
    make_shim(
        shim_dir,
        "corpus-converge",
        r#"{"converged":false,"nodes":[{"node":"alpha","lag":0},{"node":"beta","lag":5}]}"#,
    );
    make_shim(shim_dir, "corpus-arbiter", r#"{"leases":[]}"#);
    make_shim(
        shim_dir,
        "wm-tether",
        r#"{"links":[{"node":"beta","up":true,"rtt_ms":3.1}]}"#,
    );

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shim_dir.display(), old_path);

    let output = std::process::Command::new(bin_path())
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(output.status.success(), "exit 0 expected");

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let lines: Vec<&str> = stdout.lines().collect();

    // Bounded: at most 25 lines for 2-node portrait.
    assert!(
        lines.len() <= 25,
        "text view should fit in 25 lines, got {}: {}",
        lines.len(),
        stdout
    );

    // Names both nodes.
    assert!(stdout.contains("alpha"), "must name alpha");
    assert!(stdout.contains("beta"), "must name beta");

    // Attested status present.
    assert!(stdout.contains("attested"), "must show attested");

    // Session count present.
    assert!(stdout.contains("sessions"), "must show sessions");

    // Convergence verdict present.
    assert!(
        stdout.contains("converged") || stdout.contains("behind"),
        "must show convergence verdict"
    );

    // beta has lag=5, so it should show "behind".
    assert!(stdout.contains("behind"), "beta lag should show 'behind'");
}
