# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added
- Provider collection cadence cache to keep frequent scans responsive.
- Bounded local log scan controls:
  - `AGENTPULSE_LOCAL_LOG_MAX_FILES`
  - `AGENTPULSE_LOCAL_LOG_MAX_BYTES`
- Window-aware tail parsing for Codex session token usage.
- Shared command/path utilities for safer MCP command probing.
- Git status probe error capture and dashboard alert surfacing.
- Release support docs: `SECURITY.md`, `SUPPORT.md`.
- Release checksum verification script: `scripts/verify_release_assets.sh`.

### Changed
- Homebrew formula now includes stable tag/revision pin plus `head`.
- Release docs now require explicit macOS signing/notarization decision per release.

## [0.1.0] - 2026-03-02

### Added
- Initial public release of AgentPulse.
- Real-time TUI dashboard for multi-repo git health and actions.
- Worktree, process, dependency, env audit, MCP health, and AI cost views.
- Typed action execution with confirmation-first flow.
- CLI one-shot modes (`--once`, `--summary`, `--dashboard-json`, agent outputs).
