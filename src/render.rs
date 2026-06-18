//! Rendering the `WholeSelf` into human-readable output formats.

use crate::model::{FacetStatus, WholeSelf};

/// Print the human-readable self-portrait to stdout.
#[allow(clippy::print_stdout)]
pub(crate) fn print_text(whole: &WholeSelf) {
    let node_count = whole.nodes.len();
    let converged_str = if whole.converged { "converged" } else { "NOT converged" };

    println!("=== Wintermute Self-Portrait ({}) ===", whole.generated_ts);
    println!("Nodes: {node_count} | Memory: {converged_str} | Leases: {}", whole.leases.len());
    println!();

    for node in &whole.nodes {
        let attest_str = if node.attested { "attested" } else { "unattested" };
        let link_str = match &node.link {
            Some(l) if l.up => {
                l.rtt_ms.map_or_else(|| "up".to_owned(), |rtt| format!("up ({rtt:.1}ms)"))
            }
            Some(_) => "down".to_owned(),
            None => "link-unknown".to_owned(),
        };
        let sessions_str =
            node.sessions.map_or_else(|| "sessions-unknown".to_owned(), |s| format!("{s} sessions"));
        let lag_str = match node.version_lag {
            Some(0) => "current".to_owned(),
            Some(n) => format!("{n} commits behind"),
            None => "version-unknown".to_owned(),
        };

        println!(
            "  {node}: [{attest_str}] link={link_str} {sessions_str} mem={lag_str}",
            node = node.node,
            attest_str = attest_str,
            link_str = link_str,
            sessions_str = sessions_str,
            lag_str = lag_str,
        );
    }

    if !whole.leases.is_empty() {
        println!();
        println!("Leases:");
        for lease in &whole.leases {
            let expires = lease.expires.as_deref().unwrap_or("no-expiry");
            println!("  {} held by {} (expires: {})", lease.key, lease.holder, expires);
        }
    }

    // Degraded facets.
    let degraded: Vec<_> = whole
        .facet_status
        .iter()
        .filter(|(_, s)| **s != FacetStatus::Ok)
        .collect();
    if !degraded.is_empty() {
        println!();
        println!("Degraded facets:");
        for (name, status) in &degraded {
            let reason = match status {
                FacetStatus::Degraded => "not installed",
                FacetStatus::Error => "error querying",
                FacetStatus::Unknown => "unknown",
                FacetStatus::Ok => "ok",
            };
            println!("  {name}: {reason}");
        }
    }
}

/// Print the selfreview block for the self-review playbook.
#[allow(clippy::print_stdout)]
pub(crate) fn print_selfreview(whole: &WholeSelf) {
    let node_count = whole.nodes.len();
    let converged_str = if whole.converged { "yes" } else { "no" };
    let lease_count = whole.leases.len();

    println!("## corpus-introspect selfreview");
    println!("generated_ts: {}", whole.generated_ts);
    println!("node_count: {node_count}");
    println!("converged: {converged_str}");
    println!("lease_count: {lease_count}");

    for node in &whole.nodes {
        let attest_str = if node.attested { "attested" } else { "unattested" };
        println!(
            "node: {} status={attest_str}",
            node.node,
            attest_str = attest_str,
        );
    }

    for lease in &whole.leases {
        println!("lease: {} holder={}", lease.key, lease.holder);
    }

    let degraded_count = whole
        .facet_status
        .values()
        .filter(|s| **s != FacetStatus::Ok)
        .count();
    if degraded_count > 0 {
        println!("degraded_facets: {degraded_count}");
    }
}
