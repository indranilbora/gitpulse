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
