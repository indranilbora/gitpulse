# AgentPulse

AgentPulse is an agent-first terminal hub for managing many local Git repositories while vibe coding.

## Why AgentPulse

- Rebranded from `gitpulse` to avoid naming collisions and focus on agent workflows
- Built-in recommendation engine with per-repo `NEXT` action in TUI
- Agent-focused output modes:
  - `--agent-brief`: markdown handoff for terminal agents
  - `--agent-json`: structured action queue for automation
- `a` key toggles Agent Focus mode to show only actionable repos
- Existing fast repo monitoring and Git actions remain intact

## Installation

### Homebrew

```bash
brew tap indranilbora/gitpulse https://github.com/indranilbora/gitpulse
brew install --HEAD agentpulse
```

Upgrade:

```bash
brew update
brew upgrade --fetch-HEAD agentpulse
```

Uninstall:

```bash
brew uninstall agentpulse
brew untap indranilbora/gitpulse
```

### From source

```bash
cargo install --path .
```

### Run without installing

```bash
cargo run --release
```

## Quick start

```bash
# first run opens setup automatically when no config exists
agentpulse --setup

# interactive TUI
agentpulse

# one-shot table / JSON
agentpulse --once
agentpulse --once --json

# agent-optimized exports
agentpulse --agent-brief
agentpulse --agent-json
```

## CLI flags

- `--config <PATH>`: use a custom config path
- `--dir <PATH>` (repeatable): override configured watch directories for this run
- `--setup`: run setup wizard and save config, then exit
- `--once`: scan once and print a table, then exit
- `--json`: JSON output (requires `--once`)
- `--summary`: one-line summary (`exit 1` when actionable repos exist)
- `--agent-brief`: markdown handoff with prioritized recommendations
- `--agent-json`: structured recommendation queue for tools/agents

## Keyboard shortcuts

- `j` / `k` or arrows: move selection
- `Enter`: open repo in editor
- `o`: open repo in file manager
- `r`: refresh now
- `f`: git fetch
- `p`: git pull
- `P`: git push
- `c`: commit tracked changes (`git commit -a -m`)
- `g`: toggle group by directory
- `a`: toggle Agent Focus mode (actionable repos only)
- `/`: filter/search
- `s`: rerun setup wizard
- `?`: show help
- `q`: quit

## Configuration

Default config path:

```text
~/.config/agentpulse/config.toml
```

AgentPulse auto-migrates by reading the legacy path if present:

```text
~/.config/gitpulse/config.toml
```

Example:

```toml
watch_directories = ["~/Developer", "~/Projects", "~/repos"]
refresh_interval_secs = 60
max_scan_depth = 3
editor = "code"
show_clean = true
ignored_repos = ["archive"]
watch_mode = false
```

## Development

See `CONTRIBUTING.md` for local setup, checks, and release notes.
