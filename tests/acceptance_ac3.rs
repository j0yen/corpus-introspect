//! AC3: Graceful degradation — when one upstream shim is absent (e.g. no
//! wm-tether on PATH), that facet is reported as degraded/unknown with a
//! reason and the command still exits success; the missing facet is never
//! silently dropped.

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
fn ac3_missing_tether_degrades_gracefully() {
    let dir = TempDir::new().expect("tempdir");
    let shim_dir = dir.path();

    // All shims present EXCEPT wm-tether.
    make_shim(
        shim_dir,
        "corpus-attest",
        r#"{"this_node":"laptop","attested_nodes":["laptop"]}"#,
    );
    make_shim(
        shim_dir,
        "muster",
        r#"{"nodes":[{"node":"laptop","sessions":1}]}"#,
    );
    make_shim(
        shim_dir,
        "corpus-converge",
        r#"{"converged":true,"nodes":[{"node":"laptop","lag":0}]}"#,
    );
    make_shim(shim_dir, "corpus-arbiter", r#"{"leases":[]}"#);
    // wm-tether intentionally NOT created.

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shim_dir.display(), old_path);

    // Test text output.
    let output = std::process::Command::new(bin_path())
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(
        output.status.success(),
        "must exit 0 even with missing tether\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Test JSON output — facet_status must include tether as degraded.
    let json_output = std::process::Command::new(bin_path())
        .arg("--json")
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect --json");

    assert!(json_output.status.success(), "json mode exit 0");

    let stdout = String::from_utf8(json_output.stdout).expect("utf8");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    // facet_status.tether must not be "ok".
    let tether_status = v["facet_status"]["tether"]
        .as_str()
        .expect("facet_status.tether must be a string");
    assert_ne!(
        tether_status, "ok",
        "tether facet must be degraded/error, not ok"
    );
    assert!(
        tether_status == "degraded" || tether_status == "error",
        "tether facet_status should be 'degraded' or 'error', got '{tether_status}'"
    );
}

#[test]
fn ac3_missing_facet_not_silently_dropped_in_text() {
    let dir = TempDir::new().expect("tempdir");
    let shim_dir = dir.path();

    // Only attest present; all others missing.
    make_shim(
        shim_dir,
        "corpus-attest",
        r#"{"this_node":"laptop","attested_nodes":["laptop"]}"#,
    );

    // Use a PATH with only the shim_dir + a very minimal system PATH
    // (no /usr/bin etc.) — we want to ensure none of the other CLIs are found.
    let new_path = format!("{}:/usr/bin:/bin", shim_dir.display());

    let output = std::process::Command::new(bin_path())
        .env("PATH", &new_path)
        .output()
        .expect("run corpus-introspect");

    assert!(output.status.success(), "exit 0 even with most facets missing");

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    // The text output should mention degraded facets.
    assert!(
        stdout.contains("Degraded"),
        "text output must mention degraded facets\n{stdout}"
    );
}
