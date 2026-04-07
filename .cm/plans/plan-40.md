# Phase 40: Post-Run Review Agent

## Objective

After every cm run (normal exit, Ctrl-C, or crash), spawn a review agent that
inspects all logs from tasks touched during the run and prints a markdown
report to the shell. Exit non-zero if the agent flags any issues.

## Background

Today, cm runs to completion and exits silently on success. Subtle problems —
orchestration bugs, tasks marked completed despite incomplete work, unhandled
edge cases visible only in agent transcripts — are easy to miss because nobody
reads the per-phase logs. A post-run review pass closes that gap by feeding the
session's logs back to a Claude agent and surfacing anomalies before the user
moves on.

## Files to Read

- `src/manager/mod.rs`
- `src/manager/state.rs`
- `src/agent/mod.rs`
- `src/agent/prompt.rs`
- `src/agent/response.rs`
- `src/log/phase_logger.rs`
- `src/log/file_logger.rs`
- `src/main.rs`
- `src/cli/mod.rs`
- `src/lib.rs`

## Implementation Steps

### Task 1 (impl-1): Run-tracking state

1. In `src/manager/state.rs`:
   - Add `run_started_at: chrono::DateTime<chrono::Utc>` field, initialized
     to `Utc::now()` in `ManagerState::new`.
   - Add `touched_phases: std::collections::HashSet<String>` field,
     initialized empty.
   - Add `pub fn mark_phase_touched(&mut self, phase_id: &str)`.
2. In `src/manager/mod.rs`, every place the orchestration loop processes a
   task (implem spawn, build verify, review spawn, fix cycle, re-verify),
   call `state.mark_phase_touched(phase_id)` once per task touch.
3. These fields are runtime-only — do NOT persist them to `tasks.json`.

### Task 2 (impl-2): Log gathering module

1. Create `src/review/mod.rs` and declare `pub mod review;` in `src/lib.rs`.
2. Define `pub enum ReviewError` (thiserror) with at least an `Io` variant.
3. Implement
   `pub fn gather_logs(cm_dir: &Path, touched: &HashSet<String>, since: DateTime<Utc>) -> Result<String, ReviewError>`:
   - For each phase id in `touched`, read `<cm_dir>/logs/<phase_id>.log` if
     it exists; skip missing files.
   - Read `<cm_dir>/cm.log`, slice to lines whose timestamp (parsed via
     `crate::log::phase_logger::parse_log_timestamp`) is `>= since`. Keep
     lines with unparseable timestamps if they immediately follow a kept
     line (continuation lines).
   - Concatenate everything into one string with clear section headers like
     `===== phase-N.log =====` and `===== cm.log (run window) =====`.
4. Add unit tests covering: missing phase log skipped, time slicing of
   cm.log, multiple phases concatenated.

### Task 3 (impl-3): Run-review agent kind

1. In `src/agent/mod.rs`:
   - Extend the agent kind enum with a `RunReview` variant.
   - Add a spawn path for it that reuses the REVIEW model from
     `ManagerConfig` (no new config field).
2. In `src/agent/prompt.rs`:
   - Add `pub fn build_run_review_prompt(logs: &str) -> String` that
     instructs the agent to look for orchestration bugs, tasks marked
     completed when they should not be, missing edge cases, and other
     anomalies. Tell it to respond with free-form markdown, then a single
     final line containing `{"issues_found": true}` or
     `{"issues_found": false}`.
3. In `src/agent/response.rs`:
   - Add a parser that extracts the trailing JSON line and returns
     `{ markdown: String, issues_found: bool }`. If the trailing JSON is
     missing or malformed, treat as `issues_found: true` (fail safe).

### Task 4 (impl-4): Orchestrator integration + exit code

1. In `src/manager/mod.rs`, add
   `fn run_post_run_review(&self) -> bool` that:
   - Calls `review::gather_logs(cm_dir, &state.touched_phases, state.run_started_at)`.
   - If empty (no touched phases), returns `false` and prints nothing.
   - Spawns the `RunReview` agent with the gathered logs.
   - Prints the agent's markdown report to stdout verbatim (no banner, no
     color).
   - Writes the same markdown to
     `.cm/reports/run-<UTC-timestamp>.md` (creating `.cm/reports/` if
     needed). Filename format: `run-YYYYMMDD-HHMMSS.md`.
   - Returns the parsed `issues_found`.
   - Wraps every fallible step in best-effort guards: any error logs a
     warning via the existing logger and returns `false`. Must NEVER panic.
2. Invoke `run_post_run_review` from:
   - The normal end-of-`run()` success path.
   - The graceful-shutdown / Ctrl-C handler.
   - The crash / error-bail path (wherever the manager currently exits on
     unrecoverable error). Best-effort — must run even if state is partial.
3. In `src/cli/mod.rs`, return the `bool` from `run` up to `main`.
4. In `src/main.rs`, when the bool is `true`, exit process with code `1`
   AFTER the report has been fully printed.

### Task 5 (test-1): Tests

1. Unit tests for `review::gather_logs` (see Task 2).
2. Unit test for the trailing-JSON parser in `src/agent/response.rs`:
   true / false / missing / malformed.
3. Ensure `cargo build`, `cargo clippy -- -D warnings`, and `cargo test`
   all pass.

### Task 6 (review): Final review

1. Verify all success criteria below.
2. Verify the post-run review is best-effort on every shutdown path
   (no panics, no blocked exit).
3. Verify the REVIEW model is reused, no new config field added.
4. Verify report is printed verbatim (no extra formatting) AND saved to
   file.
5. Verify exit code is 1 on `issues_found == true`, 0 otherwise.

## Files to Modify

- `src/manager/state.rs` — add run_started_at, touched_phases, helper.
- `src/manager/mod.rs` — track touched phases, add `run_post_run_review`,
  invoke on normal + crash + Ctrl-C paths.
- `src/agent/mod.rs` — add `RunReview` agent kind + spawn path.
- `src/agent/prompt.rs` — `build_run_review_prompt`.
- `src/agent/response.rs` — parse `issues_found` trailing JSON.
- `src/main.rs` — propagate exit code.
- `src/cli/mod.rs` — return verdict from `run`.
- `src/lib.rs` — `pub mod review;`.

## Files to Create

- `src/review/mod.rs` — log gathering + `ReviewError`.

## Key Decisions

- **Reuse REVIEW model from config** — no new config field.
- **Markdown printed as-is**, no banner, no color.
- **Always run, even on crash** — wrapped in best-effort guards; never
  panics, never blocks shutdown.
- **Exit non-zero on any issue** — single-bit verdict via trailing JSON.
- **Touched-phase tracking** rather than time-based — covers all
  re-verifies/fix cycles cleanly.
- **Logs only** — no `tasks.json`, plans, or git diff fed to the agent.
- **Fail-safe parsing** — missing/malformed trailing JSON is treated as
  `issues_found: true` so problems are not silently swallowed.

## Verification

```
cargo build --release
cargo clippy -- -D warnings
cargo test
```

Manual:
- Run `cm` on a sample project → observe markdown report on stdout, file
  in `.cm/reports/`, exit code matches verdict.
- Ctrl-C mid-run → observe report still prints best-effort.

## Success Criteria

- [ ] After a normal `cm` run, a markdown report is printed to stdout.
- [ ] After Ctrl-C / crash, the same review runs (best-effort) and prints.
- [ ] Report is also saved to `.cm/reports/run-<timestamp>.md`.
- [ ] If the review agent reports issues, `cm` exits with code 1.
- [ ] If the review agent reports clean, `cm` exits with code 0.
- [ ] Only logs from phases touched during the run are included.
- [ ] Run-review agent uses the REVIEW model from config (no new config).
- [ ] Best-effort: never panics, never blocks shutdown.
- [ ] `cargo build`, `cargo clippy -- -D warnings`, `cargo test` all pass.

## Reviewer Criteria

**Must check:**
- [ ] `run_post_run_review` is invoked on normal exit, Ctrl-C, AND crash
      paths.
- [ ] All errors in the review path are swallowed into warnings — no
      panics.
- [ ] Trailing-JSON parser fails safe to `issues_found: true`.
- [ ] Exit code 1 propagates only after report is fully printed.
- [ ] No new fields added to `tasks.json` schema.
