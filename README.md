# corpus-introspect

**Multinode self-mirror** — synthesises attest, roster, converge, and arbiter into one view.

The wintermute corpus has four facets (attestation, roster, convergence, arbiter) but no
single place to answer "what am I, right now?" `corpus-introspect` is that mirror: one
command that assembles all five corpus facets into a single human-readable self-portrait
and machine-readable `WholeSelf` JSON record.

## Usage

```sh
# Human-readable self-portrait
corpus-introspect

# JSON output (WholeSelf record)
corpus-introspect --json

# Block suitable for the self-review playbook
corpus-introspect --format selfreview
```

## WholeSelf record

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

Any missing upstream CLI (corpus-attest, muster, corpus-converge, corpus-arbiter,
wm-tether) is reported as `"degraded"` in `facet_status` and the command still
exits 0. On a lone laptop with no corpus components installed, it reports a
single-node self (unattested, no fleet) honestly.

## Acceptance criteria

1. Given fixture shims, `--json` emits a `WholeSelf` record correctly reflecting
   nodes, leases, and converged state.
2. Text view renders a bounded self-portrait naming each node's attested+link status,
   session count, and convergence verdict.
3. When one upstream CLI is absent, that facet is `degraded` with a reason; exit 0.
4. With no corpus components, reports a single-node unattested self; no error.
5. `--format selfreview` emits a parseable block with node_count, converged, lease_count.
6. (deferred — live fleet) Two-node fleet with real RTT and session counts.
7. `cargo test` green; `sigpipe::reset()` first in `main()`; no SIGPIPE panic.

## Install

```sh
cargo install --path .
```

## Dependencies

`clap`, `serde`, `serde_json`, `sigpipe`. MSRV 1.85, edition 2021.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
