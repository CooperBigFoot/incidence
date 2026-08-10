# incidence

A substance-agnostic, event-sourced conserved-flow engine for directed compartment networks.

## Architecture

- `src/main.rs` is the composition root. It alone owns configuration, environment access, path resolution, tracing initialization, and I/O.
- `crates/core` owns domain logic. It receives typed inputs and has no configuration or I/O authority.

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo check --workspace --all-targets
cargo test --workspace
cargo build --workspace
```
