# corpus-introspect

One command that answers "what am I, right now?" for a wintermute fleet — assembling every corpus facet into a single self-portrait.

## Why it exists

The wintermute corpus knows itself through five separate facets: attestation (who is a real member), roster (who is active), convergence (is everyone's state in sync), arbiter (what leases are held), and tether (are the links up). Each facet has its own CLI. None of them answers the whole question at once. `corpus-introspect` is that mirror: it queries all five and renders one picture — human-readable for a person, a `WholeSelf` JSON record for a machine.

## Install

```sh
cargo install --path .
```

Requires Rust 1.85+. Dependencies: `clap`, `serde`, `serde_json`, `sigpipe`.

## Quickstart

```sh
corpus-introspect                      # human-readable self-portrait (default)
corpus-introspect --json               # WholeSelf record
corpus-introspect --format selfreview  # parseable block for the self-review playbook
```

The `WholeSelf` record collects per-node attestation and link health, session counts, version lag, held leases, whether the fleet has converged, and the status of each facet query:

```json
{
  "nodes": [
    {
      "node": "laptop",
      "attested": true,
      "link": { "up": true, "rtt_ms": 8.2 },
      "sessions": 2,
      "version_lag": 0
    }
  ],
  "leases": [{ "key": "lock/build", "holder": "laptop", "expires": "..." }],
  "converged": true,
  "generated_ts": "2026-06-18T00:00:00Z",
  "facet_status": {
    "attest": "ok",
    "roster": "ok",
    "converge": "ok",
    "arbiter": "ok",
    "tether": "ok"
  }
}
```

## Graceful degradation

A self-mirror has to work even when half of the self is missing. Each facet is a separate upstream CLI — `corpus-attest`, `muster` (roster), `corpus-converge`, `corpus-arbiter`, `wm-tether` — and any one that is absent is reported as `degraded` in `facet_status`, with the rest of the picture intact. The command still exits 0. On a lone laptop with no corpus components installed at all, it reports a single-node self — unattested, no fleet — and says so honestly rather than erroring.

## How it works

Five facet collectors each shell out to one upstream CLI and return structured data. A missing binary becomes a `Degraded` status, never a panic. The collected facets are merged into per-node `NodeInfo` records and a top-level `WholeSelf`, which the renderer turns into text, JSON, or the self-review block.

## Status

v0.1.0. The five facet collectors, the three output formats, and graceful degradation are tested and working. Live two-node verification with real RTT and session counts (AC6) is deferred until a real fleet is wired; today's tests run against fixture shims.

## Where it fits

The reflective layer of the wintermute fleet. It owns no state — it reads the facets that `corpus-attest`, `muster`, `corpus-converge`, `corpus-arbiter`, and `wm-tether` each own, and composes them into one view.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
