# Contributing

## Prerequisites

- Rust stable toolchain
- Git available in `PATH`

Install toolchain:

```bash
rustup toolchain install stable
rustup default stable
```

## Local development

```bash
cargo run
```

Use setup with a custom config path when testing:

```bash
cargo run -- --setup --config /tmp/gitpulse-dev.toml
cargo run -- --config /tmp/gitpulse-dev.toml
```

## Quality checks

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Project layout

- `src/main.rs`: CLI entrypoint, event loop, non-interactive output
- `src/setup.rs`: interactive setup and config writing
- `src/config.rs`: config schema and loading
- `src/scanner.rs`: repo discovery
- `src/git.rs`: status collection via git commands
- `src/monitor.rs`: scan orchestration + status cache
- `src/ui/`: ratatui rendering components
- `tests/integration.rs`: end-to-end integration tests against real repos

## Release flow

- Tag `v*` to trigger `.github/workflows/release.yml`
- Workflow builds platform binaries and publishes to crates.io
- Ensure `CARGO_REGISTRY_TOKEN` is configured in repository secrets
- Homebrew tap formula is in `Formula/gitpulse.rb`
- After tagging a new release, bump `url`, `version`, and `sha256` in `Formula/gitpulse.rb`
- Recompute Homebrew crate checksum with:
  ```bash
  VERSION=0.1.0
  cargo package --allow-dirty --no-verify --offline
  shasum -a 256 target/package/gitpulse-${VERSION}.crate
  ```
- After pushing formula updates, verify tap install with:
  ```bash
  brew untap indranilbora/gitpulse || true
  brew tap indranilbora/gitpulse https://github.com/indranilbora/gitpulse
  brew install --build-from-source gitpulse
  brew test gitpulse
  ```
