# AgentPulse Product Quality Roadmap (Solo / macOS-first)

## Constraints and Strategy

- [x] Solo developer workflow
- [x] Confirm mode by default for actions
- [x] Near real-time responsiveness prioritized
- [x] macOS-first acceptable

## Milestone 1: Safe Action Runtime (P0)

- [x] Replace string shell execution with typed actions (`enum`) and argument-safe process spawning
- [x] Remove `sh -lc` execution path for dashboard actions
- [x] Add central action dispatcher with explicit allowlisted commands
- [x] Add action confirmation modal (default `Confirm`, explicit `Run once`, `Cancel`)
- [x] Add destructive action classification (`kill`, `git rm`, etc.) with stronger confirmation copy
- [x] Add tests for action serialization/dispatch and confirmation flow defaults

Acceptance criteria:
- No dashboard action uses raw shell evaluation
- `x` always opens confirm UI before execution
- Destructive actions cannot run in one keystroke

## Milestone 2: Near Real-Time Status Accuracy (P1)

- [x] Replace `.git/index`-only cache key with multi-signal invalidation
- [x] Include HEAD/upstream reference mtimes in cache validity checks
- [x] Add short staleness TTL fallback for remote-related fields (`ahead/behind`)
- [x] Trigger selective invalidation after local actions (`fetch/pull/push/commit`)
- [x] Add tests for stale-ahead/behind scenarios and post-action freshness

Acceptance criteria:
- Ahead/behind stays current after fetch/push/pull without full cold scan
- Repo state freshness improves while scan latency remains low

## Milestone 3: Trustworthy Metrics and Alerts (P1/P2)

- [x] Fix local usage heuristic session counting (avoid per-object inflation)
- [x] Separate metrics provenance in UI: `live`, `local logs`, `heuristic`
- [x] Add `last updated` timestamps per provider source
- [x] Improve alert dedupe/sorting to surface highest-value triage items first
- [x] Add tests for provider metric parsers and fallback behavior

Acceptance criteria:
- Session/token/cost figures are materially less noisy
- User can see where each metric came from and how fresh it is

## Milestone 4: UX Polish for High-Quality Feel (P2)

- [x] Add command preview pane in confirmation modal (`what will run`, `where`, `risk level`)
- [x] Improve success/error notifications with actionable next step hints
- [x] Add compact loading states for integrations and slow collectors
- [x] Ensure zero-truncation critical information in selected-row detail area
- [x] Add keyboard help entries for confirm/cancel flow

Acceptance criteria:
- Workflow feels deliberate and safe
- Error states are diagnosable without leaving the TUI

## Milestone 5: Scope Clarity and Documentation (P2)

- [x] Document macOS-first behavior and known Linux/Windows gaps
- [x] Mark `watch_mode` as planned/experimental until implemented
- [x] Add product principles section to README (speed, safety, clarity)
- [x] Add release checklist for solo maintenance

Acceptance criteria:
- README matches actual behavior
- Product expectations are clear to contributors/users

## Suggested PR Slices

- [x] PR1: Typed action model + remove `sh -lc` + base dispatcher
- [x] PR2: Confirmation modal + destructive action policy
- [x] PR3: Cache invalidation overhaul + selective refresh hooks
- [x] PR4: Provider metrics correctness + provenance metadata
- [x] PR5: UX polish + docs alignment

## Verification Plan (for each PR)

- [x] `cargo test -q`
- [x] `cargo clippy -q`
- [x] Manual TUI walkthrough: confirm flow, repo actions, alerts, refresh behavior
- [x] Compare behavior against current `main` for regressions in scan speed and core keybindings

## Review Notes

Use this section after each PR:

- Summary:
- Risks found:
- What changed in user-visible behavior:
- Follow-up tasks:

### PR1 Review

- Summary: Replaced shell-string action execution with typed `ActionKind` payloads and centralized allowlisted dispatch in `src/actions.rs`. Dashboard and collectors now emit typed actions via `ActionCommand::new(...)`, and `x` executes typed actions.
- Risks found: Confirm mode is not implemented yet (planned in PR2), so actions still run immediately on keypress.
- What changed in user-visible behavior: Action execution messages remain similar. Action preview strings are now derived from typed actions and included in serialized dashboard output.
- Follow-up tasks: Implement PR2 confirmation modal and destructive-action gating before expanding action surface area.

### PR2 Review

- Summary: Added confirmation mode (`x` now stages action), confirmation modal with risk level + command preview, and destructive-action warning copy.
- Risks found: Commit (`c`) remains message-first and executes on Enter; this is intentional but not routed through a second confirmation modal.
- What changed in user-visible behavior: `x`, `f`, `p`, and `P` no longer execute instantly; Enter/y confirms, Esc/n cancels.
- Follow-up tasks: Optionally add a final commit confirmation after message entry if you want strict two-step safety.

### PR3 Review

- Summary: Replaced index-only cache checks with multi-signal git metadata checks (`index`, `HEAD`, `FETCH_HEAD`, remote refs) plus bounded staleness TTL and selective invalidation after action-triggering keys.
- Risks found: Refresh-after-action is optimistic (triggered immediately after launch), so long-running commands may need one extra refresh cycle.
- What changed in user-visible behavior: Faster status freshness after fetch/pull/push/commit and safer cache reuse under polling.
- Follow-up tasks: Optional post-action completion-triggered refresh channel for exact timing.

### PR4 Review

- Summary: Fixed local session inflation heuristic, added provider data provenance + updated timestamp fields, surfaced both in AI Costs table, and deduped/sorted alerts by severity/actionability.
- Risks found: Local fallback still relies on heuristic parsing of heterogeneous logs; accuracy depends on source format consistency.
- What changed in user-visible behavior: AI Costs now shows source (`live/local_logs/heuristic/unconfigured`) and update recency, with cleaner alert feed.
- Follow-up tasks: Add stricter schema-aware parsers for known local log formats.

### PR5 Review

- Summary: Added selected-row detail strip for all non-home sections, integration loading states while scanning, improved action notifications, and updated README/CONTRIBUTING scope and release guidance.
- Risks found: Detail strip is concise plain text and can still be clipped by terminal width.
- What changed in user-visible behavior: Better situational context, safer action UX, clearer docs about macOS-first scope and watch mode status.
- Follow-up tasks: Add horizontal scrolling or expandable detail modal for very long lines.

### Follow-up Fix (Post PR5)

- Summary: Switched refresh behavior from launch-time to completion-time for confirmed actions and commit runs.
- Risks found: Multiple fast completions can queue multiple scans (bounded by existing scan throughput).
- What changed in user-visible behavior: Repo/process/dependency state updates land immediately after action completion instead of one refresh cycle later.
- Follow-up tasks: Add scan coalescing for bursts of completed actions.

### Follow-up Fix 2 (Post PR5)

- Summary: Added scan coalescing so bursty action completions and repeated `r` key presses enqueue at most one additional scan while a scan is in flight.
- Risks found: Back-to-back scans still occur when queue drains, which is expected for freshness.
- What changed in user-visible behavior: Status bar shows `Refresh queued`, and scan churn is reduced under rapid refresh/action bursts.
- Follow-up tasks: Optional debounce window if users still trigger too many rescans.

### Manual TUI Walkthrough Notes (2026-03-01)

- Confirm flow (`x` -> modal -> `n`) works in Worktrees and Home alert rows.
- Confirm run (`x` -> `y`) executes typed action and returns notification with command result.
- `r` spam during active scan now queues refresh instead of launching parallel scans.
- Help overlay (`?`) reflects confirm mode shortcuts accurately.

## Market Readiness Audit (2026-03-02)

### Completed Investigation

- [x] Run quality gate checks (`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`)
- [x] Run CLI smoke checks (`--once`, `--summary`, `--dashboard-json`)
- [x] Profile snapshot latency on real local state
- [x] Review release/distribution pipeline and packaging metadata
- [x] Identify code-level risks with line-level evidence

### Confirmed Issues To Resolve Before Broad Launch

- [x] P0: Reduce dashboard/summary latency caused by provider log scanning every refresh
  - Evidence:
  - `target/debug/agentpulse --summary --dir /Users/indranilbora/gitpulse` takes ~1.94s to ~6.42s in this environment.
  - With empty home (`HOME=/tmp/agentpulse-empty-home`), same command drops to ~0.01s to ~0.02s.
  - Root cause path: `collect_all()` always calls `collect_provider_usage()` (`src/collectors/mod.rs:27-40`), which rescans and parses local provider logs on every pass (`src/collectors/ai_mcp.rs:228-291`, `src/collectors/ai_mcp.rs:1148-1246`).
- [x] P1: Fix contradictory provider note when supplemental local data is found
  - Evidence:
  - Claude row can show both `source: ~/.claude/stats-cache.json` and `no local usage logs found in common paths` in one payload.
  - Root cause path: note is added when `log_files` is empty (`src/collectors/ai_mcp.rs:335-337`) and not removed after Claude/Codex supplementary local sources set `has_local_data = true` (`src/collectors/ai_mcp.rs:293-318`).
- [x] P1: Harden MCP command health checks for quoted paths and Windows PATH behavior
  - Evidence:
  - Command parsing uses `split_whitespace()` (`src/collectors/ai_mcp.rs:1299-1303`), which breaks quoted binary paths.
  - PATH lookup only checks literal `dir.join(binary)` (`src/collectors/ai_mcp.rs:1326-1331`, `src/actions.rs:265-270`), which misses `PATHEXT` behavior on Windows.
- [x] P2: Surface git probe failures instead of silently flattening to clean-ish defaults
  - Evidence:
  - `check_repo_status()` suppresses per-probe errors and returns fallback values (`src/git.rs:162-165`), which can hide status collection failures from users.

### Execution Plan (Market-Ready Track)

- [ ] Phase 1: Performance and trust fixes (3-4 days)
  - [x] Split provider collection cadence from repo status cadence (slow lane for provider data, fast lane for repo status).
  - [x] Add bounded local log scan limits (`AGENTPULSE_LOCAL_LOG_MAX_FILES`, `AGENTPULSE_LOCAL_LOG_MAX_BYTES`) to cap worst-case scan time.
  - [x] Replace Codex full-file scans with window-aware tail parsing (`month-to-date`/lookback aware) for session usage.
  - [x] Add regression benchmark target: `--summary` under 300ms for 1-20 repos on warm cache.
  - [x] Remove contradictory notes when supplementary local sources are present.
  - [ ] Add provider collector cache keyed by source mtimes + lookback window (optional follow-up if cadence cache is insufficient).
- [x] Phase 2: Reliability hardening (2-3 days)
  - [x] Replace whitespace command split with shell-like parsing for MCP configs.
  - [x] Expand binary resolution for Windows extensions and executable checks.
  - [x] Emit explicit alerts when git status probes fail (instead of silent default fields).
- [x] Phase 3: Release readiness and packaging (2 days)
  - [x] Add stable Homebrew formula path (non-HEAD) and release artifact verification steps.
  - [x] Add `CHANGELOG.md`, `SECURITY.md`, and support policy docs for external users.
  - [x] Add release validation step for macOS binary notarization/signing decision.
- [x] Phase 4: Launch validation (2 days)
  - [x] Manual smoke on representative macOS setups (small, medium, large workspace).
  - [x] Validate first-run setup UX from a clean machine profile.
  - [x] Validate upgrade path from legacy `~/.config/gitpulse/config.toml`.

### Verification Gate For Market Launch

- [x] Functional: `cargo test --locked` passes on CI and local.
- [x] Quality: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [x] Performance: `--summary` and initial TUI snapshot meet latency budget on warm cache.
- [x] Safety: destructive actions always require explicit confirmation.
- [ ] Distribution: tagged release installs cleanly via crates.io + Homebrew stable formula.
- [x] Docs: README/CONTRIBUTING/release docs match real behavior and supported platforms.

### Review Notes (2026-03-02)

- Summary: Build is healthy and tests pass, but performance and trust issues remain in provider collection and MCP command validation.
- Risks found: slow summary/dashboard refresh under real home-directory telemetry data; contradictory provider notes; brittle command/binary checks on non-trivial paths.
- User-visible impact: users may experience multi-second refresh, confusing AI cost notes, and false MCP unhealthy signals.
- Recommended next action: execute Phase 1 and Phase 2 before announcing broader adoption.

### Review Notes (2026-03-02, Phase 1 Execution Update)

- Summary: Implemented provider refresh cadence caching, bounded generic log scans, and fast Codex token extraction via tail reads scoped to active report window.
- Performance result: `--summary` moved from ~1.94s-6.42s to ~0.43s cold and ~0.04s warm in this environment; `--dashboard-json` is now ~0.04s-0.05s on repeated runs.
- Trust result: removed contradictory `no local usage logs found` notes when supplementary local data is successfully used.
- Remaining work: Phase 2 reliability hardening (MCP command parsing/path resolution and explicit git probe failure surfacing).

### Review Notes (2026-03-02, Phase 2 Execution Update)

- Summary: Added shared command/binary path utilities, moved MCP health checks to shell-like command parsing, and surfaced git probe degradation through dashboard alerts.
- Reliability result: quoted/assignment-prefixed MCP commands resolve to the correct executable token; PATH lookup now handles executable semantics more robustly.
- Observability result: non-fatal git probe failures are captured and elevated as high-severity actionable alerts instead of being silent.
- Remaining work: Phase 3 release packaging/docs hardening and Phase 4 launch validation.

### Review Notes (2026-03-02, Phase 3 Execution Update)

- Summary: Added stable Homebrew formula pin (tag/revision), release asset checksum verification script, and policy docs (`CHANGELOG.md`, `SECURITY.md`, `SUPPORT.md`).
- Release pipeline result: release workflow now smoke-tests binaries, publishes source archive/checksum, and publishes Homebrew metadata snippet for stable formula updates.
- Governance result: release checklist now requires explicit macOS signing/notarization decision before release publication.
- Remaining work: Phase 4 launch validation on representative machine/workspace profiles.

### Review Notes (2026-03-02, Phase 4 Execution Update)

- Summary: Ran launch validation against small/medium/large workspaces plus clean-profile first-run and legacy config migration flows.
- Workspace validation:
  - Small (`/Users/indranilbora/gitpulse`): `--once`, `--summary`, and `--dashboard-json` all returned expected actionable state.
  - Medium (`/tmp/agentpulse-phase4-medium`, 12 repos): one-shot and dashboard flows succeeded; actionable/dirty counts matched seeded repo state.
  - Large (`/tmp/agentpulse-phase4-large`, 60 repos): one-shot and dashboard flows succeeded; seeded dirty repos surfaced correctly.
- Performance validation:
  - Medium `--summary`: ~0.70s cold, ~0.28s-0.30s warm.
  - Large `--summary`: ~0.95s cold, ~0.54s-0.82s warm.
- First-run validation:
  - Clean HOME auto-run path showed welcome + setup wizard, wrote config at `~/.config/agentpulse/config.toml`, then attempted TUI startup.
- Legacy migration validation:
  - With only `~/.config/gitpulse/config.toml`, app used legacy config successfully.
  - Running setup persisted equivalent config to `~/.config/agentpulse/config.toml`.
- Environment note:
  - Interactive TUI smoke in this sandbox exits with `Operation not permitted (os error 1)` after setup; this is environment/PTY restricted and should be rechecked in a normal local terminal before public announcement.

## Dedicated Landing Page Launch Plan (2026-03-02)

### Implementation Checklist

- [x] Create static website scaffold under `website/` (`index.html`, `styles.css`, `main.js`, assets, README).
- [x] Implement dedicated landing sections (hero, capabilities, safety, speed, OSS, install, FAQ, footer) with install-only CTA strategy.
- [x] Enforce design constraints (<=3 colors, no gradients, large hit targets, mobile/desktop responsive).
- [x] Add privacy-first analytics event contract (`lp_view` + CTA events) with UTM metadata forwarding.
- [x] Add Vercel deployment and analytics setup instructions in `website/README.md`.
- [x] Validate anchors, links, keyboard navigation, and install command accuracy.

### Review Notes

- Summary: Added a fully static dedicated landing page under `website/` with install-only CTAs, analytics contract wiring, brand assets, and deployment docs.
- Risks found: Browser-specific rendering and Lighthouse thresholds still require manual verification in Chrome/Safari/Firefox on a real host build.
- User-visible behavior: Primary CTA scrolls to install, install commands are copyable, and docs/support/security/footer links point to project docs.
- Follow-up tasks: Run cross-browser manual QA on deployed Vercel URL, then validate event ingestion in Vercel Analytics dashboard.

## Shiori-Style Landing Redesign (2026-03-04)

### Implementation Checklist

- [x] Rebuild `website/index.html` with new IA: header, hero, demo, features, install, footer.
- [x] Replace `website/styles.css` with Shiori-style token system and responsive card layout.
- [x] Keep copy actions and command palette flows with updated section anchors.
- [x] Update `website/main.js` analytics contract to `lp_view` and `lp_click_install_primary` only.
- [x] Update `website/README.md` to reflect new IA, style direction, and event contract.
- [x] Validate local preview behavior on `http://127.0.0.1:4173`.
- [x] Verify command-copy controls and keyboard palette shortcuts.
- [x] Verify no extra analytics events are emitted from landing script.

### Verification Checklist

- [x] Primary CTA links to `#install`.
- [x] Homebrew copy button reads from `#cmd-homebrew`.
- [x] Run-once copy button reads from `#cmd-run`.
- [x] `Cmd/Ctrl+K` opens `dialog#commandPalette` and `Esc` closes.
- [x] `main.js` only emits `lp_view` and `lp_click_install_primary`.

### Review Notes

- Summary: Rebuilt the landing page to a Shiori-style visual system with a full IA rewrite (hero, demo window, feature grid, install cards, minimal footer), while retaining copy actions and command palette behavior.
- Risks found: Browser-level visual QA (desktop/mobile rendering nuances) still needs manual confirmation in a real browser session.
- User-visible behavior changes: New premium, centered layout and component styling; minimal analytics contract now emits only `lp_view` and `lp_click_install_primary`.
- Follow-up tasks: Run manual visual QA at 1440px and 375px on Chrome/Safari/Firefox and confirm event ingestion in Vercel dashboard.
