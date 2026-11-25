# Replica Integration for Replica Project

## Quick Start
```bash
# Build the base (one-time)
replica base build

# Create a workspace
cd $(replica new --ttl 2h)

# Run the replica binary from the workspace
./target/release/replica --help
```

## Development
- Run tests: `cargo test`
- Build release: `cargo build --release`
- Run clippy: `cargo clippy`
- Run fmt: `cargo fmt`

## Configuration
- REPLICA_ROOT is configured via `.replica/config.json`
- Base repo: local path
- Setup command: `cargo build --release`