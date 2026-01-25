# Build & Test Actions

## Build

```bash
cargo build --release
```

Produces the release binary at `target/release/cm`.

## Test

```bash
cargo test
```

Runs all integration tests in `tests/integration.rs`.

## Lint

```bash
cargo clippy
```

Must pass clean (no warnings) before committing.

## All Checks

All three checks must pass before committing:

```bash
cargo build --release && cargo clippy && cargo test
```

## Install

```bash
just install
```

Builds and installs to `/usr/bin/cm` (requires sudo).

## Other Commands

```bash
just build          # Same as cargo build --release
just --list         # Show all available just commands
```
