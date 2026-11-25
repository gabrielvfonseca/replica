# Replica Skill for Replica Development

## When to use replica
- Working on the replica codebase
- Testing workspace isolation features
- Benchmarking clone performance

## Commands
- `replica new` - Create workspace
- `replica base build` - Build base
- `replica gc` - Cleanup
- `cargo test` - Run tests

## Configuration
- REPLICA_ROOT from .replica/config.json
- Base: local path with cargo build --release