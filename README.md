# AgentPulse

AgentPulse is a real-time terminal dashboard for your local development environment.

## What It Monitors

- Git repo health: dirty/ahead/behind, detached state, stash, and recommended next actions
- Git worktrees across scanned repos
- Repo-scoped running processes
- Dependency hygiene across Node, Rust, Python, Go, and Ruby projects
- `.env` audit (metadata only): missing/extra keys and tracked sensitive env files
- MCP server configuration + command health checks
- AI provider setup/usage/cost rollups for Claude, Gemini, and OpenAI (live provider APIs when configured, with local-log fallback)
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

## Real AI Usage and Cost Data

AgentPulse now prefers live provider data and falls back to local log heuristics if live access is not configured.

- Default window: month-to-date
- Override window: `AGENTPULSE_COST_LOOKBACK_DAYS=<N>`
- API polling cache: `AGENTPULSE_PROVIDER_CACHE_SECS=60` (default)
- API timeout: `AGENTPULSE_PROVIDER_TIMEOUT_SECS=8` (default)
- Pagination cap: `AGENTPULSE_PROVIDER_MAX_PAGES=6` (default)

### OpenAI (organization APIs)

Set an org admin key (falls back to `OPENAI_API_KEY` if set):

```bash
export OPENAI_ADMIN_KEY=...
```

Direct terminal checks:

```bash
END_EPOCH="$(date -u +%s)"
START_EPOCH="$((END_EPOCH - 86400))"

curl -sS https://api.openai.com/v1/organization/usage/completions \
  -H "Authorization: Bearer $OPENAI_ADMIN_KEY" \
  -G --data-urlencode "start_time=$START_EPOCH" \
  --data-urlencode "end_time=$END_EPOCH" \
  --data-urlencode "bucket_width=1d" \
  --data-urlencode "limit=31"

curl -sS https://api.openai.com/v1/organization/costs \
  -H "Authorization: Bearer $OPENAI_ADMIN_KEY" \
  -G --data-urlencode "start_time=$START_EPOCH" \
  --data-urlencode "end_time=$END_EPOCH" \
  --data-urlencode "bucket_width=1d" \
  --data-urlencode "limit=31"
```

### Anthropic (organization reports)

Set an org admin key (falls back to `ANTHROPIC_API_KEY` if set):

```bash
export ANTHROPIC_ADMIN_API_KEY=...
```

Direct terminal checks:

```bash
START_RFC3339="$(date -u -v-30d +%Y-%m-%dT00:00:00Z)"  # 30 days ago
END_RFC3339="$(date -u +%Y-%m-%dT%H:%M:%SZ)"          # now

curl -sS https://api.anthropic.com/v1/organizations/usage_report/messages \
  -H "x-api-key: $ANTHROPIC_ADMIN_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -G --data-urlencode "starting_at=$START_RFC3339" \
  --data-urlencode "ending_at=$END_RFC3339" \
  --data-urlencode "bucket_width=1d" \
  --data-urlencode "limit=31"

curl -sS https://api.anthropic.com/v1/organizations/cost_report \
  -H "x-api-key: $ANTHROPIC_ADMIN_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -G --data-urlencode "starting_at=$START_RFC3339" \
  --data-urlencode "ending_at=$END_RFC3339" \
  --data-urlencode "bucket_width=1d" \
  --data-urlencode "limit=31"
```

### Gemini / Google

Gemini API does not currently expose an org-level usage/cost endpoint equivalent to OpenAI/Anthropic.
AgentPulse can pull real Gemini cost totals from Google Cloud Billing export in BigQuery:

```bash
export AGENTPULSE_GEMINI_BQ_TABLE="my-project.billing_export.gcp_billing_export_v1_xxxxx"
```

Prerequisites:

- `bq` CLI installed and authenticated (part of the [Google Cloud SDK](https://cloud.google.com/sdk/docs/install))
- Cloud Billing export enabled to BigQuery

Optional filter override:

```bash
# Default filter matches Generative Language, Vertex AI, and Gemini SKUs.
# Override to narrow or expand:
export AGENTPULSE_GEMINI_BQ_SERVICE_FILTER="LOWER(sku.description) LIKE '%gemini%'"
```

## Development

See `CONTRIBUTING.md` for local setup, checks, and release notes.
