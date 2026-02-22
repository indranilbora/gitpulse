# GitPulse

GitPulse is a terminal UI for monitoring many local Git repositories in one place.

## Features

- Recursive repo discovery across configured directories
- Color-coded status for dirty, unpushed, and clean repositories
- Filter/search, grouping by parent directory, and keyboard-first navigation
- Quick actions: open in editor, open in file manager, fetch, pull, push, commit
- First-run setup wizard and reusable config file
- Non-interactive modes for scripts: `--once`, `--json`, and `--summary`

## Installation

### Homebrew

```bash
brew tap indranilbora/gitpulse https://github.com/indranilbora/gitpulse
brew install gitpulse
```

Upgrade:

```bash
brew update
brew upgrade gitpulse
```

Uninstall:

```bash
brew uninstall gitpulse
brew untap indranilbora/gitpulse
```

Use development `main` branch build:

```bash
brew install --HEAD gitpulse
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
cargo run -- --setup

# start TUI
cargo run

# one-shot output for shell scripts
cargo run -- --once
cargo run -- --once --json
cargo run -- --summary
```

## CLI flags

- `--config <PATH>`: use a custom config path
- `--dir <PATH>` (repeatable): override configured watch directories for this run
- `--setup`: run setup wizard and save config, then exit
- `--once`: scan once and print a table, then exit with code `1` if any repo needs attention
- `--json`: JSON output (requires `--once`)
- `--summary`: one-line summary, exits `1` when any repo is dirty

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
- `/`: filter/search
- `s`: rerun setup wizard
- `?`: show help
- `q`: quit

## Configuration

Default config path:

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
