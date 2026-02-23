# AgentPulse

AgentPulse is a real-time terminal dashboard for your local development environment.

## What It Monitors

- Git repo health: dirty/ahead/behind, detached state, stash, and recommended next actions
- Git worktrees across scanned repos
- Repo-scoped running processes
- Dependency hygiene across Node, Rust, Python, Go, and Ruby projects
- `.env` audit (metadata only): missing/extra keys and tracked sensitive env files
- MCP server configuration + command health checks
- AI provider setup/usage/cost rollups for Claude, Gemini, and OpenAI (from local config/log hints)
- Unified alerts with executable actions (`x` runs the selected action)

## Installation

### Homebrew

```bash
brew tap indranilbora/agentpulse https://github.com/indranilbora/agentpulse
brew install --HEAD indranilbora/agentpulse/agentpulse
```

Upgrade:

```bash
brew update
brew upgrade --fetch-HEAD indranilbora/agentpulse/agentpulse
```

Uninstall:

```bash
brew uninstall indranilbora/agentpulse/agentpulse
brew untap indranilbora/agentpulse
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
agentpulse --dashboard-json

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
- `--dashboard-json`: full snapshot including repos, processes, deps, env audit, MCP, and AI usage/cost

## Keyboard shortcuts

- `h` / `l` or `Tab`: switch dashboard section
- `1..8`: jump to section
- `j` / `k` or arrows: move selection in current section
- `x`: run selected row action
- `Enter`: open repo in editor (Repos section)
- `o`: open repo in file manager (Repos section)
- `r`: refresh now
- `f`: git fetch (Repos section)
- `p`: git pull (Repos section)
- `P`: git push (Repos section)
- `c`: commit tracked changes (Repos section)
- `g`: toggle group by directory (Repos section)
- `A`: toggle actionable-only repo mode (Repos section)
- `/`: filter/search (Repos section)
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
refresh_interval_secs = 10
max_scan_depth = 3
editor = "code"
show_clean = true
ignored_repos = ["archive"]
watch_mode = false
```

## Development

See `CONTRIBUTING.md` for local setup, checks, and release notes.
