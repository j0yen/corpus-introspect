//! Facet collectors: each function queries one upstream CLI and returns
//! structured data. All degrade gracefully — a missing binary means a
//! `FacetStatus::Degraded` entry, never a panic.

use std::collections::HashMap;

use crate::model::{FacetStatus, LeaseInfo, LinkInfo};

// ---------------------------------------------------------------------------
// Return types for each facet
// ---------------------------------------------------------------------------

/// Data returned by the attest facet.
pub struct AttestData {
    /// (node_name, is_attested) pairs.
    pub nodes: Vec<(String, bool)>,
}

/// Data returned by the roster facet.
pub struct RosterData {
    /// (node_name, active_session_count) pairs.
    pub sessions: Vec<(String, u32)>,
}

/// Data returned by the converge facet.
pub struct ConvergeData {
    /// Is the fleet fully converged?
    pub converged: bool,
    /// (node_name, version_lag_commits) pairs. 0 = up to date.
    pub version_lags: Vec<(String, u32)>,
}

/// Data returned by the arbiter facet.
pub struct ArbiterData {
    /// Currently held leases.
    pub leases: Vec<LeaseInfo>,
}

/// Data returned by the tether facet.
pub struct TetherData {
    /// (node_name, link_info) pairs.
    pub links: Vec<(String, LinkInfo)>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run a subprocess and return its stdout as a String, or None if the binary
/// is not found / exits non-zero.
fn run_cli(args: &[&str]) -> Result<String, CliError> {
    let (binary, rest) = args.split_first().ok_or(CliError::NotFound)?;
    let output = std::process::Command::new(binary)
        .args(rest)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CliError::NotFound
            } else {
                CliError::IoError(e.to_string())
            }
        })?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| CliError::ParseError(e.to_string()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(CliError::ExitError(
            output.status.code().unwrap_or(-1),
            stderr,
        ))
    }
}

#[derive(Debug)]
enum CliError {
    NotFound,
    IoError(String),
    ExitError(i32, String),
    ParseError(String),
}

impl CliError {
    fn to_facet_status(&self) -> FacetStatus {
        match self {
            CliError::NotFound => FacetStatus::Degraded,
            _ => FacetStatus::Error,
        }
    }
}

// ---------------------------------------------------------------------------
// Facet: attest
// ---------------------------------------------------------------------------

/// Collect attestation data from `corpus-attest --json`.
///
/// Expected JSON shape:
/// ```json
/// { "this_node": "laptop", "attested_nodes": ["laptop", "server"] }
/// ```
pub fn collect_attest(status: &mut HashMap<String, FacetStatus>) -> AttestData {
    match run_cli(&["corpus-attest", "--json"]) {
        Ok(stdout) => {
            match parse_attest_json(&stdout) {
                Ok(data) => {
                    status.insert("attest".to_owned(), FacetStatus::Ok);
                    data
                }
                Err(e) => {
                    status.insert("attest".to_owned(), FacetStatus::Error);
                    eprintln!("corpus-introspect: attest parse error: {e}");
                    AttestData { nodes: vec![] }
                }
            }
        }
        Err(e) => {
            status.insert("attest".to_owned(), e.to_facet_status());
            AttestData { nodes: vec![] }
        }
    }
}

fn parse_attest_json(s: &str) -> Result<AttestData, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| e.to_string())?;
    let mut nodes = Vec::new();

    if let Some(this) = v.get("this_node").and_then(|n| n.as_str()) {
        // Mark this node as attested if it appears in attested_nodes.
        let attested_nodes: Vec<String> = v
            .get("attested_nodes")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();

        let this_attested = attested_nodes.contains(&this.to_owned());
        nodes.push((this.to_owned(), this_attested));

        for n in &attested_nodes {
            if n != this {
                nodes.push((n.clone(), true));
            }
        }
    }

    Ok(AttestData { nodes })
}

// ---------------------------------------------------------------------------
// Facet: roster (muster --fleet --json)
// ---------------------------------------------------------------------------

/// Collect roster / active-session data from `muster --fleet --json`.
///
/// Expected JSON shape:
/// ```json
/// { "nodes": [{ "node": "laptop", "sessions": 3 }] }
/// ```
pub fn collect_roster(status: &mut HashMap<String, FacetStatus>) -> RosterData {
    match run_cli(&["muster", "--fleet", "--json"]) {
        Ok(stdout) => {
            match parse_roster_json(&stdout) {
                Ok(data) => {
                    status.insert("roster".to_owned(), FacetStatus::Ok);
                    data
                }
                Err(e) => {
                    status.insert("roster".to_owned(), FacetStatus::Error);
                    eprintln!("corpus-introspect: roster parse error: {e}");
                    RosterData { sessions: vec![] }
                }
            }
        }
        Err(e) => {
            status.insert("roster".to_owned(), e.to_facet_status());
            RosterData { sessions: vec![] }
        }
    }
}

fn parse_roster_json(s: &str) -> Result<RosterData, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| e.to_string())?;
    let mut sessions = Vec::new();

    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
        for node in nodes {
            let name = node
                .get("node")
                .and_then(|n| n.as_str())
                .ok_or_else(|| "missing node.node".to_owned())?;
            let count = node
                .get("sessions")
                .and_then(|s| s.as_u64())
                .unwrap_or(0) as u32;
            sessions.push((name.to_owned(), count));
        }
    }

    Ok(RosterData { sessions })
}

// ---------------------------------------------------------------------------
// Facet: converge (corpus-converge version --json)
// ---------------------------------------------------------------------------

/// Collect convergence state from `corpus-converge version --json`.
///
/// Expected JSON shape:
/// ```json
/// { "converged": true, "nodes": [{ "node": "laptop", "lag": 0 }] }
/// ```
pub fn collect_converge(status: &mut HashMap<String, FacetStatus>) -> ConvergeData {
    match run_cli(&["corpus-converge", "version", "--json"]) {
        Ok(stdout) => {
            match parse_converge_json(&stdout) {
                Ok(data) => {
                    status.insert("converge".to_owned(), FacetStatus::Ok);
                    data
                }
                Err(e) => {
                    status.insert("converge".to_owned(), FacetStatus::Error);
                    eprintln!("corpus-introspect: converge parse error: {e}");
                    ConvergeData {
                        converged: false,
                        version_lags: vec![],
                    }
                }
            }
        }
        Err(e) => {
            status.insert("converge".to_owned(), e.to_facet_status());
            ConvergeData {
                converged: false,
                version_lags: vec![],
            }
        }
    }
}

fn parse_converge_json(s: &str) -> Result<ConvergeData, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| e.to_string())?;
    let converged = v
        .get("converged")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);

    let mut version_lags = Vec::new();
    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
        for node in nodes {
            let name = node
                .get("node")
                .and_then(|n| n.as_str())
                .ok_or_else(|| "missing node.node".to_owned())?;
            let lag = node.get("lag").and_then(|l| l.as_u64()).unwrap_or(0) as u32;
            version_lags.push((name.to_owned(), lag));
        }
    }

    Ok(ConvergeData {
        converged,
        version_lags,
    })
}

// ---------------------------------------------------------------------------
// Facet: arbiter (corpus-arbiter status --json)
// ---------------------------------------------------------------------------

/// Collect held leases from `corpus-arbiter status --json`.
///
/// Expected JSON shape:
/// ```json
/// { "leases": [{ "key": "lock/build", "holder": "laptop", "expires": "..." }] }
/// ```
pub fn collect_arbiter(status: &mut HashMap<String, FacetStatus>) -> ArbiterData {
    match run_cli(&["corpus-arbiter", "status", "--json"]) {
        Ok(stdout) => {
            match parse_arbiter_json(&stdout) {
                Ok(data) => {
                    status.insert("arbiter".to_owned(), FacetStatus::Ok);
                    data
                }
                Err(e) => {
                    status.insert("arbiter".to_owned(), FacetStatus::Error);
                    eprintln!("corpus-introspect: arbiter parse error: {e}");
                    ArbiterData { leases: vec![] }
                }
            }
        }
        Err(e) => {
            status.insert("arbiter".to_owned(), e.to_facet_status());
            ArbiterData { leases: vec![] }
        }
    }
}

fn parse_arbiter_json(s: &str) -> Result<ArbiterData, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| e.to_string())?;
    let mut leases = Vec::new();

    if let Some(arr) = v.get("leases").and_then(|l| l.as_array()) {
        for item in arr {
            let key = item
                .get("key")
                .and_then(|k| k.as_str())
                .ok_or_else(|| "missing lease.key".to_owned())?
                .to_owned();
            let holder = item
                .get("holder")
                .and_then(|h| h.as_str())
                .unwrap_or("unknown")
                .to_owned();
            let expires = item
                .get("expires")
                .and_then(|e| e.as_str())
                .map(str::to_owned);
            leases.push(LeaseInfo { key, holder, expires });
        }
    }

    Ok(ArbiterData { leases })
}

// ---------------------------------------------------------------------------
// Facet: tether (wm-tether status --json)
// ---------------------------------------------------------------------------

/// Collect link health from `wm-tether status --json`.
///
/// Expected JSON shape:
/// ```json
/// { "links": [{ "node": "server", "up": true, "rtt_ms": 12.4 }] }
/// ```
pub fn collect_tether(status: &mut HashMap<String, FacetStatus>) -> TetherData {
    match run_cli(&["wm-tether", "status", "--json"]) {
        Ok(stdout) => {
            match parse_tether_json(&stdout) {
                Ok(data) => {
                    status.insert("tether".to_owned(), FacetStatus::Ok);
                    data
                }
                Err(e) => {
                    status.insert("tether".to_owned(), FacetStatus::Error);
                    eprintln!("corpus-introspect: tether parse error: {e}");
                    TetherData { links: vec![] }
                }
            }
        }
        Err(e) => {
            status.insert("tether".to_owned(), e.to_facet_status());
            TetherData { links: vec![] }
        }
    }
}

fn parse_tether_json(s: &str) -> Result<TetherData, String> {
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| e.to_string())?;
    let mut links = Vec::new();

    if let Some(arr) = v.get("links").and_then(|l| l.as_array()) {
        for item in arr {
            let node = item
                .get("node")
                .and_then(|n| n.as_str())
                .ok_or_else(|| "missing link.node".to_owned())?
                .to_owned();
            let up = item.get("up").and_then(|u| u.as_bool()).unwrap_or(false);
            let rtt_ms = item.get("rtt_ms").and_then(|r| r.as_f64());
            links.push((node, LinkInfo { up, rtt_ms }));
        }
    }

    Ok(TetherData { links })
}
