# Phase 41: am producer socket

## Objective

Let the `am` fleet monitor follow a running cm. While cm runs agents (default
run, `--step`, `--continue`), it listens on its own Unix socket and streams
newline-delimited JSON (NDJSON) frames to every connected client:

- a copy of every line cm prints to stdout/stderr;
- typed events at each orchestration step (phase started, agent spawned,
  verdict, failure, waiting on stdin);
- on connect, a replay of current state and recent output.

`am-server` (one per machine) discovers the socket by scanning a directory.
cm never connects anywhere and never blocks on a client.

## Background

Today cm only reports progress on the terminal (`emit_cm`, `tee_*`) and in
`.cm/logs/cm.log`. Nothing outside the terminal can see which phase is
running, which agent step it is on, or that cm is blocked on a stdin prompt
(`[s]ingle / [a]ll / ...` or `Retry more cycles?`).

The monitor side is designed in the `am` repository (`docs/producers.md`,
`docs/cm-integration.md`). This phase implements the producer contract below.
Frame shapes, socket naming and directory rules must match it exactly.

### Contract

**Socket path.** `<dir>/cm-<pid>.sock`, `<pid>` is `std::process::id()`.
`<dir>` is resolved in this order:

1. `AM_SOCKET_DIR` environment variable, if set and non-empty;
2. `[am] socket_dir` in the cm config file;
3. `$XDG_RUNTIME_DIR/am`, if `XDG_RUNTIME_DIR` is set and non-empty.

If none resolves, or `[am] enabled = false`, the integration is off: no
socket, every send is a no-op. When no dir resolves, log once with
`log::warn!`. `enabled` defaults to `true`.

`<dir>` is created with mode `0700` if missing. If a file already exists at
the socket path (stale file from a crashed process with the same pid), it is
removed before binding.

**Frames.** One JSON object per line, UTF-8, `\n` terminated. Optional fields
are omitted when unset (never `null`).

```json
{"type":"event","kind":"progress","source":"cm","pid":1234,"task":"phase-41 PHASE_REVIEW","message":"Phase review cycle 2/5 for phase-41","fields":{"phase":"phase-41","step":"PHASE_REVIEW","cycle":2}}
{"type":"output","source":"cm","pid":1234,"seq":17,"stream":"stderr","text":"[2026-09-17T10:00:00Z CM] Selected phase: phase-41"}
{"type":"state","source":"cm","pid":1234,"task":"phase-41 PHASE_REVIEW","attention":true,"attention_message":"Task phase-41 exhausted 5 review cycles. Retry more cycles?"}
```

- `event`: `kind` (required), `source` (required, `"cm"`), `pid` (required),
  `message` (required), `task`, `pct`, `level`, `fields` (optional). cm never
  sets `pct` or `level` (the server derives level from kind).
- `output`: `source`, `pid`, `seq`, `stream` (`"stdout"` or `"stderr"`),
  `text` (one line, no trailing newline, `\r` removed, trailing whitespace
  trimmed). `seq` is 1 for the first output line of the cm process and is
  incremented per output line; replayed backlog frames keep their original
  `seq`, so `am-server` can skip lines it already has after a reconnect.
- `state`: `source`, `pid`, `task` (optional), `attention` (optional bool,
  omitted when false), `attention_message` (optional).

**Kinds used by cm.** `started`, `progress`, `completed`, `deferred`,
`failed`, `needs_attention`, `resolved`.

**`fields` keys used by cm.** `phase` (phase id), `step` (agent label or
`BUILD`), `cycle` (1-based cycle number), `verdict` (`"approved"` or
`"needs_fixes"`). Only keys that are known at the call site are set.

**Connection behaviour.**

- One accept thread. On accept, cm writes, in order: one `state` frame, then
  the output backlog (last `OUTPUT_BACKLOG_LINES = 10000` output frames, each
  with its original `seq`), then the client joins the live list.
- Each client stream has a write timeout of `WRITE_TIMEOUT = 100 ms`. A write
  that fails or times out drops that client. Other clients are unaffected.
- cm never reads from clients.
- The socket file is removed when cm exits through a normal return or through
  an explicit `std::process::exit` in the CLI. A panic, SIGKILL or SIGTERM
  leave the file behind; `am-server` removes stale files when connect is
  refused.

**Tracked state** (what the `state` frame replays):

- `task`: set by `started` and `progress` events to their `task`; cleared by
  `completed`, `deferred`, `failed`.
- `attention` / `attention_message`: set by `needs_attention` (message is the
  event message), cleared by `resolved`.

**Not forwarded.** The per-second `\r` redraw ticker in
`src/agent/mod.rs:322` and the line clear at `src/agent/mod.rs:311` stay
terminal-only. They redraw one line in place and are not in `cm.log` either.

## Files to Read

- `src/lib.rs`
- `src/config/mod.rs`
- `src/cli/mod.rs`
- `src/manager/mod.rs`
- `src/log/mod.rs`
- `src/log/tee_writer.rs`
- `src/agent/mod.rs`
- `CONFIG.md`
- `assets/templates/config.toml`

## Implementation Steps

### Task 1 (impl-1): `[am]` config

1. In `src/config/mod.rs`:
   - Add `pub struct AmConfig` (`Debug, Clone, Default, Serialize,
     Deserialize`, `#[serde(default)]`) with `pub enabled: Option<bool>` and
     `pub socket_dir: Option<PathBuf>`, next to `PriceTable` (`:34`).
   - Add `pub am: Option<AmConfig>` to `ConfigFile` (`:66-90`) with
     `#[serde(skip_serializing_if = "Option::is_none")]`, same pattern as
     `pricing`.
   - Add `&& self.am.is_none()` to `is_empty()` (`:136`).
   - Add `am: None` to the struct literal in `test_config_file_serialize`
     (`:267`).
2. In `src/manager/mod.rs`:
   - Add `pub am_socket_dir: Option<PathBuf>` to `ManagerConfig` (`:102`),
     `None` in `ManagerConfig::new`, and a builder
     `pub fn am_socket_dir(mut self, dir: Option<PathBuf>) -> Self` after
     `build_commands` (`:160`). `None` means the integration is off.
3. In `src/cli/mod.rs`, `build_manager_config` (`:314-382`):
   - After the config file values are applied, resolve the socket dir:
     if `[am] enabled == Some(false)`, `None`; else `AM_SOCKET_DIR` env if
     non-empty, else `[am] socket_dir`, else `$XDG_RUNTIME_DIR` joined with
     `am` if non-empty, else `None`
     with one `warn!("am: no socket dir (set AM_SOCKET_DIR or XDG_RUNTIME_DIR), integration off")`.
   - Put the resolution in a small function
     `fn resolve_am_socket_dir(file: Option<&AmConfig>, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf>`
     so it can be tested without touching the process environment.
4. `CONFIG.md`: add `[am]` to the options table (`enabled`, `socket_dir`),
   describe the resolution order and the socket path `<dir>/cm-<pid>.sock`.
5. `assets/templates/config.toml`: add a commented `[am]` section with both
   keys and the default resolution in a comment.

### Task 2 (impl-2): `src/am/mod.rs` socket module

1. Create `src/am/mod.rs`, declare `pub mod am;` in `src/lib.rs` with a doc
   line (`/// Producer socket for the am fleet monitor.`).
2. Constants: `OUTPUT_BACKLOG_LINES: usize = 10000`,
   `WRITE_TIMEOUT: Duration = Duration::from_millis(100)`,
   `SOURCE: &str = "cm"`.
3. Types:
   - `enum Stream { Stdout, Stderr }` serialized lowercase.
   - `enum EventKind { Started, Progress, Completed, Deferred, Failed,
     NeedsAttention, Resolved }` serialized snake_case.
   - `struct EventFields { phase: Option<String>, step: Option<String>,
     cycle: Option<u32>, verdict: Option<Verdict> }`, each field
     `skip_serializing_if = "Option::is_none"`; `Verdict` is
     `crate::state::Verdict`, whose serde form is already `"approved"` /
     `"needs_fixes"` (`src/state/tasks.rs:318-320`).
   - `enum Frame` tagged `#[serde(tag = "type", rename_all = "snake_case")]`
     with `Event { .. }`, `Output { source, pid, seq: u64, stream, text }`,
     `State { .. }` matching the contract. Frames are built only through
     this enum and `serde_json`.
4. `pub struct Producer { inner: Arc<Mutex<Inner>> }` with `Inner { path:
   PathBuf, clients: Vec<UnixStream>, next_seq: u64, backlog:
   VecDeque<BacklogLine>, task: Option<String>, attention: Option<String>,
   partial_stdout: Vec<u8>, partial_stderr: Vec<u8> }`, where
   `BacklogLine { seq: u64, stream: Stream, text: String }` and `next_seq`
   starts at 1. Steps 5 to 10 live on `Producer`, except `start`. The
   global is `static AM: OnceLock<Producer>`, same `OnceLock` pattern as
   `LOG_FILE` in `src/log/tee_writer.rs:12`; free functions `start`,
   `send_event`, `send_output`, `send_output_line`, `shutdown` forward to it
   and are no-ops when it is not set (integration off).
5. `Producer::bind(dir: &Path) -> io::Result<Producer>`:
   - Create dir with `std::fs::DirBuilder` + `DirBuilderExt::mode(0o700)`,
     `recursive(true)`; remove an existing file at the socket path; bind
     `UnixListener`.
   - Spawn the accept thread (it holds a clone of the `Arc`). For each
     accepted stream: set write timeout, lock, write the `state` frame and
     the backlog as `output` frames with their stored `seq`; on success push
     to `clients`, on failure drop it.
   - `pub fn start(dir: Option<&Path>)`: `None` or already started: return;
     `bind` error: `warn!` once and return (integration off); else store in
     `AM`.
6. `send_event(kind: EventKind, task: Option<&str>, message: &str,
   fields: EventFields)`: update tracked `task` / `attention` per the
   contract, then broadcast.
7. `send_output(stream: Stream, bytes: &[u8])`: append `bytes` to the
   stream's partial buffer, emit one `output` frame per complete line
   (decoded with `String::from_utf8_lossy`, `\r` removed, trailing
   whitespace trimmed), keep the remainder. Bytes, not `&str`, so a UTF-8
   character split across two `TeeWriter::write` calls is not mangled. Each
   line takes `seq = next_seq` and increments `next_seq`; the line is pushed
   to `backlog` with its `seq` (pop front beyond `OUTPUT_BACKLOG_LINES`) and
   broadcast.
8. `send_output_line(stream: Stream, bytes: &[u8])`: same as `send_output`,
   then flush the remaining partial for that stream as its own line. Used by
   `tee_print` for prompts that have no trailing newline (the task selection
   prompt starts with `\n`, so it gives an empty line then the prompt
   line).
9. Broadcast: serialize once, append `\n`, `write_all` to each client, drop
   clients whose write errors (timeouts included). Lock poisoning: return
   silently. Nothing in this module returns an error or panics to callers.
   No `log` macro may run while the lock is held: `env_logger` writes through
   `TeeWriter`, which calls `send_output`, which takes the same lock
   (deadlock). Collect what to log, release the lock, then log.
10. `shutdown()`: if started, remove the socket file. Clients are left
    to see EOF when the process exits.
11. Start the socket in `src/cli/mod.rs` right after each
    `build_manager_config(cli)?` in agent-running modes:
    `execute_run` (`:389`), the three `execute_continue` branches (`:445`,
    `:470`, `:502`), `execute_step` (`:526`):
    `crate::am::start(config.am_socket_dir.as_deref());`.
12. Remove the socket on exit:
    - Add `fn exit_process(code: i32) -> !` in `src/cli/mod.rs` that calls
      `crate::am::shutdown()` then `std::process::exit(code)`, and use it for
      every `std::process::exit` in `execute_run` (`:395`, `:398`),
      `execute_continue` (`:452`, `:455`, `:477`, `:480`, `:509`, `:512`)
      and `execute_step` (`:533`, `:536`).
    - Call `crate::am::shutdown()` in `run()` before `result` is returned
      (after the `match &result` at `:292`). Ctrl-C only sets the shutdown
      flag (`src/manager/recovery.rs:411`), so it exits through these paths.

### Task 3 (impl-3): copy stdout/stderr to the socket

1. In `src/log/tee_writer.rs`:
   - `tee_println` (`:68`): `crate::am::send_output(Stream::Stdout, s)` then
     a `"\n"` (or one call with `format!("{s}\n")`), as bytes.
   - `tee_print` (`:74`): `crate::am::send_output_line(Stream::Stdout,
     s.as_bytes())`.
   - `tee_eprintln` (`:83`): same as `tee_println` on `Stream::Stderr`.
   - `impl Write for TeeWriter` (`:109`): after the terminal write, pass
     `&buf[..written]` to `send_output` on the matching stream.
     This covers every `log`/`env_logger` record (`src/cli/mod.rs:221-223`).
2. Direct print calls that bypass tee while agents run. Route each through
   the tee functions so the socket copy matches the terminal (cm.log gains
   these lines too):
   - `src/agent/mod.rs:334` agent completion line: the call site writes the
     `\r` and padding erase of the ticker line to the terminal only, with a
     raw `eprint!`, and passes the clean line text
     (`[<ts> <label>] <task_id> completed in <N>s`, no `\r`, no padding) to
     `tee_eprintln`, so `cm.log` and the socket get a clean line.
   - `src/manager/mod.rs:1903` run review report: `tee_println`.
   - `src/cli/mod.rs:394` `Run error`, `:451`, `:476`, `:508`
     `Continue error`, `:532` `Step error`: `tee_eprintln`.
3. Left as is:
   - `src/agent/mod.rs:311` and `:322` (redraw ticker, terminal-only).
   - `src/cli/mod.rs:217` (before the log file exists, before the socket).
   - `src/main.rs:10` (after `cli::run` returned, socket already removed).
   - Prints in `src/cli/init.rs`, `src/cli/token.rs`,
     `src/cli/update_pricing.rs` and the non-agent `execute_*` modes: the
     socket is never started there.

### Task 4 (impl-4): events at orchestration sites

All in `src/manager/mod.rs` unless noted. `task` is `"<phase-id> <STEP>"`
for step events and `"<phase-id>"` for phase events. `message` reuses the
text of the neighbouring `emit_cm` when there is one.

Phase `started` is sent only through
`fn emit_phase_started(&mut self, phase_id: &str)`, which sends `started`
(task `<phase-id>`, fields `phase`, message `"Executing phase: <id>"`)
unless `am_started_phase == Some(phase_id)`, then sets `am_started_phase`
to `Some(phase_id)`. `am_started_phase: Option<String>` is a new `Manager`
field, `None` in `Manager::new`. An attempt is one call of `execute_phase`
from `run()`, `run_specific_phases` or `step()`: each of these sets
`am_started_phase = None` just before that call, so the recursive
`execute_phase` at `:691` does not send a second `started`.

1. Phase start, after `mark_phase_status(phase_id, InProgress)` (`:716`):
   `emit_phase_started`.

   Phase with no pending tasks (`if pending_tasks.is_empty()` block in
   `execute_phase`, `:660-712`) sends the same events as a normal phase:
   - on entry, before `check_phase_completion` (`:663`):
     `emit_phase_started`;
   - build step, just before `verify_commands` in `check_phase_completion`
     (`:1643`): `progress`, task `<phase-id> BUILD`, fields `phase`, step
     `BUILD`;
   - outcome, just before each return: `completed` (fields `phase`) at
     `:669` and `:710`, `deferred` (fields `phase`, message the neighbouring
     log text `Phase <id> has deferred tasks, marking phase Deferred`) at
     `:672` and `:702`;
   - errors (every `?` in the block and the `Err` at `:698`) surface from
     `execute_phase` and get the single `failed` of item 4.
   The re-execute at `:691` goes through the normal path, where
   `emit_phase_started` sends nothing: `started` was already sent in this
   attempt.
2. Agent and build steps, just before each call: `progress`, fields
   `phase`, `step`, and `cycle` where the loop has one (1-based):
   - PLAN spawn `:781` (step `PHASE_PLAN`);
   - IMPLEM spawn `:864` (step `PHASE_IMPLEM`);
   - build check after IMPLEM `:993` (step `BUILD`);
   - REVIEW spawn in `run_phase_review_cycle` `:1141` (step
     `PHASE_REVIEW`, cycle);
   - FIX spawn `:1280` (step `PHASE_FIX`, cycle);
   - build after FIX `:1322` (step `BUILD`, cycle);
   - build-fix FIX spawn in `run_build_fix_loop` `:1403` (step `PHASE_FIX`,
     cycle);
   - build-fix build check `:1432` (step `BUILD`, cycle);
   - run review spawn in `run_post_run_review` `:1877` (task `run-review
     RUN_REVIEW`, step `RUN_REVIEW`, no phase).

   After the run review agent returns and its response is parsed, just
   before `response.has_issues` is returned (`:1919`), same task and step,
   no phase:
   `completed` with message `"Run review: no issues found"` when
   `has_issues` is false, `failed` with message `"Run review: issues found"`
   when it is true.

   Spawn or wait error in `run_post_run_review` (`Err` arms at `:1879` and
   `:1887`), before the `return false`: `failed`, same task and step, no
   phase, message the error text of the neighbouring `warn!`
   (`"run_post_run_review: failed to spawn agent: <e>"` or
   `"run_post_run_review: agent error: <e>"`).
3. Verdicts in `apply_phase_verdict` (`:1036`), the single place verdicts
   are applied. Emit just before each `Ok(PhaseOutcome::..)` (`:1053`,
   `:1068`), after every `?` in that arm, so an error there gives only the
   `failed` event of item 4:
   - `Verdict::Approved`: `completed`, fields `phase`, `verdict: approved`;
   - `Verdict::NeedsFixes`: `deferred`, fields `phase`,
     `verdict: needs_fixes`, message the existing deferral reason.
4. Failures. Agent-reported failure (`:953`), build-fix exhausted (`:1457`)
   and every other phase error surface as `Err` from `execute_phase`. Emit
   one `failed` (task `<phase-id>`, fields `phase`, message
   `"Phase <id> failed: <error>"`) where that `Err` is handled, so it is sent
   exactly once. It goes through
   `fn emit_phase_failed(&mut self, phase_id: &str, err: &ManagerError)`,
   which calls `emit_phase_started` first, so `started` precedes `failed`
   even when the error happened before any `started` site (for example
   `mark_phase_status` failing at `:716`):
   - `run()` `:318-321`;
   - `run_specific_phases` `:591-593`;
   - `step()` `:363`: replace the `?` with `inspect_err` that emits, then
     propagate.
   `ShutdownRequested` is not a failure: no event.
5. Stdin prompts:
   - `prompt_task_selection`: after the stdout flush (`:447-449`, which can
     return early with `?`), `needs_attention` with message `"Select tasks:
     [s]ingle / [a]ll / [p]hase / [q]uit"`; after `read_line` (`:455`)
     returns, ok or error, `resolved`, before the `?` propagates the error.
   - `prompt_retry_cycles`: before `tee_print` at `:616`,
     `needs_attention` with the existing `message` (`:610`), fields `phase`,
     `cycle`; after `read_line` (`:621`) returns (ok or error), `resolved`.

### Task 5 (test-1): tests

Unit tests at the end of `src/am/mod.rs` and `src/cli/mod.rs`, using
`tempfile::TempDir` and `serde_json::Value` to parse frames. The global
`OnceLock` can only be set once per test binary, so tests build their own
`Producer` with `Producer::bind` on a fresh `TempDir` and never call
`start`.

1. Frame serialization: each `Frame` variant produces the exact JSON of the
   contract; optional fields absent when unset; `kind`, `stream`, `verdict`
   spellings.
2. Line splitting: `"a\nb"` then `"c\n"` gives lines `a`, `bc`; `\r` removed;
   trailing spaces trimmed; a multi-byte UTF-8 character split across two
   calls comes out intact; `send_output_line("x")` after a pending `"a"`
   gives one line `ax`; `send_output_line("\nq: ")` gives lines `""`, `q:`.
3. Backlog: after 10100 lines, a new client receives the `state` frame then
   exactly the last 10000 output lines in order, then live frames.
   Replay keeps `seq`: the replayed frames carry `seq` 101 to 10100, the
   same values they had when first sent, and the next live line has `seq`
   10101.
   The first output line of a fresh `Producer` has `seq` 1.
4. State replay: `progress` then `needs_attention` gives a `state` frame with
   `task` and `attention: true`; after `resolved`, `attention` is absent;
   after `completed`, `task` is absent. Run review: `progress` with task
   `run-review RUN_REVIEW` then `completed`, or then `failed`, gives a
   `state` frame without `task`.
5. No client: sends return and nothing is kept except backlog and state.
6. Dropped client: a client that closes its end is removed on the next send;
   a second client keeps receiving; no panic.
7. Stuck client: a connected client that never reads does not block sends
   beyond the write timeout and is then dropped (fill its buffer with large
   frames, assert the send loop finishes and the client count drops).
8. Socket file: created at `<dir>/cm-<pid>.sock`, dir mode `0700`, a
   pre-existing file at that path is replaced, file removed by shutdown.
9. Config: `[am]` with both keys parses; `is_empty()` false when only `[am]`
   is set; `resolve_am_socket_dir` covers env set, file set, XDG only,
   nothing set, empty `AM_SOCKET_DIR` falls through, `enabled = false` with
   env set.

### Task 6 (review): Final review

1. Frames, socket path, dir resolution and connect replay match the
   Contract section exactly.
2. A client that stops reading blocks a send for about `WRITE_TIMEOUT`
   before it is dropped, and no path panics or returns an error from
   `crate::am`.
3. Every `std::process::exit` in agent-running modes goes through
   `exit_process`.
4. `failed` is emitted once per failed phase and always after a `started`
   for that phase; no attempt sends `started` twice; `needs_attention` is
   always followed by `resolved` on both prompts.
5. With the integration off (no dir, or `enabled = false`), behaviour and
   output are unchanged.

## Files to Modify

- `src/lib.rs`: `pub mod am;`.
- `src/config/mod.rs`: `AmConfig`, `ConfigFile.am`, `is_empty`, test.
- `src/manager/mod.rs`: `ManagerConfig.am_socket_dir`, `am_started_phase`,
  events, report print.
- `src/cli/mod.rs`: dir resolution, `start`, `exit_process`, `shutdown`,
  error prints.
- `src/log/tee_writer.rs`: forward to the socket.
- `src/agent/mod.rs`: completion line through tee.
- `CONFIG.md`, `assets/templates/config.toml`: `[am]`.

## Files to Create

- `src/am/mod.rs`: socket, frames, state, tests.

## Key Decisions

- **cm listens, am-server connects.** cm owns no reconnect logic; a late or
  restarted monitor gets current state and recent output on connect.
- **Pid in the file name.** am-server attributes the socket to the cm
  process tree without extra handshake.
- **No new dependency.** `std::os::unix::net`, `serde_json`, `log`.
- **Output copy at the tee layer.** Everything that reaches `cm.log` reaches
  the socket; the redraw ticker reaches neither.
- **Failures emitted at the `execute_phase` callers**, not at each inner
  error site, so each failed phase produces one event.

## Verification

```
cargo build --release
cargo clippy --all-targets -- -D warnings
cargo test
```

Manual:
- `AM_SOCKET_DIR=/tmp/amtest cm` (default run, which shows the task
  selection prompt) in a sample project, then
  `socat - UNIX-CONNECT:/tmp/amtest/cm-<pid>.sock`: a `state` frame, the
  backlog, then live `output` and `event` frames.
- Answer the task selection prompt: `needs_attention` then `resolved`.
- Kill the `socat` client mid-run: cm carries on.
- After cm exits, `/tmp/amtest/cm-<pid>.sock` is gone.

## Success Criteria

- [ ] cm creates `<dir>/cm-<pid>.sock` in run, step and continue modes when
      a dir resolves and `[am] enabled` is not false.
- [ ] Connected clients receive the state replay, the output backlog, then
      live frames matching the contract.
- [ ] Every line written through tee and env_logger is sent as an `output`
      frame.
- [ ] Events are emitted at every site listed in Task 4, including the
      phase path with no pending tasks (`started`, `progress` for the build,
      `completed` / `deferred`, `failed` on error).
- [ ] The post-run review sends `completed` when the review agent finds no
      issues and `failed` when it finds issues or fails to spawn or wait.
- [ ] Every phase `failed` is preceded by one `started` for that phase in
      the same attempt.
- [ ] Every `output` frame carries `seq`, starting at 1 and incremented per
      line; replayed backlog frames keep their original `seq`.
- [ ] A slow or dead client never blocks or crashes cm.
- [ ] The socket file is removed on normal exit and on CLI `exit` paths.
- [ ] `cargo build`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test` all pass.

## Reviewer Criteria

**Must check:**
- [ ] JSON field names and kind spellings match the contract.
- [ ] Optional fields are omitted, never `null`.
- [ ] A client that stops reading is dropped after about `WRITE_TIMEOUT`;
      the lock is not held longer than that per such client.
- [ ] No `unwrap`/`expect` in `src/am/mod.rs` outside tests.
- [ ] No `log` macro is called while the `crate::am` lock is held.
- [ ] Integration off leaves terminal output and `cm.log` identical to
      before, except the lines newly routed through tee in Task 3.
