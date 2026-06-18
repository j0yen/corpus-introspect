//! Data model for the `WholeSelf` record emitted by corpus-introspect.

use serde::{Deserialize, Serialize};

use crate::facets::{collect_arbiter, collect_attest, collect_converge, collect_roster, collect_tether};

/// Status of a single corpus facet query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacetStatus {
    /// Data collected successfully.
    Ok,
    /// Facet CLI is not installed.
    Degraded,
    /// Facet CLI ran but returned an error.
    Error,
    /// Facet data is absent for an unknown reason.
    Unknown,
}

/// Per-node information assembled from all facets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier (hostname or NATS subject prefix).
    pub node: String,
    /// Whether this node has a valid corpus attestation.
    pub attested: bool,
    /// Link health from `wm-tether` (None if tether unavailable).
    pub link: Option<LinkInfo>,
    /// Active session count from `muster` (None if roster unavailable).
    pub sessions: Option<u32>,
    /// Version lag in commits behind the converged state (None if converge unavailable).
    pub version_lag: Option<u32>,
}

/// Link health information from `wm-tether`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    /// Is the link currently up?
    pub up: bool,
    /// Round-trip time in milliseconds, if measured.
    pub rtt_ms: Option<f64>,
}

/// A held lease from `corpus-arbiter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    /// Lease key / name.
    pub key: String,
    /// Which node holds this lease.
    pub holder: String,
    /// When the lease expires (ISO 8601), if known.
    pub expires: Option<String>,
}

/// The complete self-portrait: all nodes, leases, and whether memory is converged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WholeSelf {
    /// All known nodes that constitute the entity.
    pub nodes: Vec<NodeInfo>,
    /// Currently held leases across the fleet.
    pub leases: Vec<LeaseInfo>,
    /// Is the entire fleet converged on the same memory state?
    pub converged: bool,
    /// Timestamp when this record was generated (ISO 8601).
    pub generated_ts: String,
    /// Facet collection status — degraded facets are named here.
    pub facet_status: std::collections::HashMap<String, FacetStatus>,
}

impl WholeSelf {
    /// Collect all corpus facets and synthesise a `WholeSelf` record.
    ///
    /// Never panics — missing CLIs degrade gracefully to `FacetStatus::Degraded`.
    #[must_use]
    pub fn collect() -> Self {
        let mut facet_status = std::collections::HashMap::new();

        let attest = collect_attest(&mut facet_status);
        let roster = collect_roster(&mut facet_status);
        let converge = collect_converge(&mut facet_status);
        let arbiter = collect_arbiter(&mut facet_status);
        let tether = collect_tether(&mut facet_status);

        // Build node list: start from attested set, merge roster + tether data.
        let mut node_map: std::collections::HashMap<String, NodeInfo> =
            std::collections::HashMap::new();

        // Seed from attest
        for (node_name, is_attested) in &attest.nodes {
            node_map
                .entry(node_name.clone())
                .or_insert_with(|| NodeInfo {
                    node: node_name.clone(),
                    attested: *is_attested,
                    link: None,
                    sessions: None,
                    version_lag: None,
                })
                .attested = *is_attested;
        }

        // If no attest data, at least record this node as unattested.
        if attest.nodes.is_empty() {
            let hostname = hostname_or_unknown();
            node_map.entry(hostname.clone()).or_insert(NodeInfo {
                node: hostname,
                attested: false,
                link: None,
                sessions: None,
                version_lag: None,
            });
        }

        // Merge roster session counts.
        for (node_name, session_count) in &roster.sessions {
            node_map
                .entry(node_name.clone())
                .or_insert_with(|| NodeInfo {
                    node: node_name.clone(),
                    attested: false,
                    link: None,
                    sessions: None,
                    version_lag: None,
                })
                .sessions = Some(*session_count);
        }

        // Merge tether link health.
        for (node_name, link) in &tether.links {
            node_map
                .entry(node_name.clone())
                .or_insert_with(|| NodeInfo {
                    node: node_name.clone(),
                    attested: false,
                    link: None,
                    sessions: None,
                    version_lag: None,
                })
                .link = Some(link.clone());
        }

        // Merge version lag from converge.
        for (node_name, lag) in &converge.version_lags {
            node_map
                .entry(node_name.clone())
                .or_insert_with(|| NodeInfo {
                    node: node_name.clone(),
                    attested: false,
                    link: None,
                    sessions: None,
                    version_lag: None,
                })
                .version_lag = Some(*lag);
        }

        let mut nodes: Vec<NodeInfo> = node_map.into_values().collect();
        nodes.sort_by(|a, b| a.node.cmp(&b.node));

        Self {
            nodes,
            leases: arbiter.leases,
            converged: converge.converged,
            generated_ts: utc_now(),
            facet_status,
        }
    }
}

fn hostname_or_unknown() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_owned()))
        .unwrap_or_else(|_| "localhost".to_owned())
}

fn utc_now() -> String {
    // RFC 3339 without external crate: use the system date command.
    std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_owned(), |s| s.trim().to_owned())
}
