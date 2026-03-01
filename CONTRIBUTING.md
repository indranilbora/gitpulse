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
cargo run -- --setup --config /tmp/agentpulse-dev.toml
cargo run -- --config /tmp/agentpulse-dev.toml
```

## Quality checks

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Provider cost data (optional local check)

The AI cost collector will use live provider APIs when configured and otherwise fall back to local logs.

- OpenAI: `OPENAI_ADMIN_KEY`
- Anthropic: `ANTHROPIC_ADMIN_API_KEY`
- Gemini: `AGENTPULSE_GEMINI_BQ_TABLE` (BigQuery billing export + `bq` CLI auth)

## Project layout

- `src/main.rs`: CLI entrypoint, event loop, non-interactive output
- `src/app.rs`: dashboard state, section selection, row selection, action targeting
- `src/setup.rs`: interactive setup and config writing
- `src/config.rs`: config schema and loading
- `src/scanner.rs`: repo discovery
- `src/git.rs`: status collection via git commands
- `src/monitor.rs`: scan orchestration + status cache
- `src/collectors/`: git/worktrees, AI+MCP, processes/deps/env collectors
- `src/dashboard/`: snapshot model + overview/alert builder
- `src/ui/`: ratatui rendering components
- `tests/integration.rs`: end-to-end integration tests against real repos

## Release flow

- Tag `v*` to trigger `.github/workflows/release.yml`
- Workflow builds platform binaries and publishes to crates.io
- Ensure `CARGO_REGISTRY_TOKEN` is configured in repository secrets
- Homebrew tap formula is in `Formula/agentpulse.rb`
- Formula currently tracks `HEAD` on `master`
- After pushing formula updates, verify tap install with:
  ```bash
  brew untap indranilbora/agentpulse || true
  brew tap indranilbora/agentpulse https://github.com/indranilbora/agentpulse
  brew install --HEAD indranilbora/agentpulse/agentpulse
  brew test indranilbora/agentpulse/agentpulse
  ```

## Solo Release Checklist

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] Manual TUI smoke test (`agentpulse`) with action confirm flow
- [ ] Manual one-shot checks (`--once`, `--summary`, `--dashboard-json`)
- [ ] README matches behavior (platform scope, watch mode status, shortcuts)
- [ ] Tag release and verify Homebrew install path
