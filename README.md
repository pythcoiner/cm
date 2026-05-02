# Claude Code Manager (cm)

Automated agent coordination for software development.

`cm` is an **orchestrator**. It does not improvise: it follows a structured
plan stored as JSON (`tasks.json` + `roadmap.json`) and drives a strict
PLAN -> IMPLEM -> BUILD -> REVIEW -> FIX cycle on every phase, spawning
Claude Code agents in full isolation for each step. The wizards (`/cm`,
`/feat`, `/split`, `/end`, ...) turn your intent into that structured plan;
`cm` then executes it autonomously, end to end, with a complete audit
trail.

## Disclaimer & Security

**Read this before running `cm` on anything you care about.**

- **This project was written entirely by an LLM.** The author has not
  reviewed the code at all - not line by line, not at a high level, not
  even casually. There is no audit, no formal review, and no guarantee
  the implementation actually does what the README says it does.
- **Do not trust the author, and do not trust the LLM that produced
  this.** Treat the binary the same way you would treat any unaudited
  tool that runs shell commands and edits files in your repo.
- **`cm` spawns Claude Code with `--dangerously-skip-permissions`.**
  Every agent (PLAN, IMPLEM, REVIEW, FIX) runs in Claude Code's
  bypass-permissions mode. There are **no per-tool prompts**, no
  approval checkpoints, no chance to deny a file write or a shell
  command. The agents have unrestricted use of every Claude Code tool
  for the entire run.
- **Run on a dedicated machine, in a VM, or inside an isolated
  sandbox.** Combined with the bypass-permissions mode above, this
  means a bug in the orchestrator, a prompt-injection attack against
  any agent, or a malicious dependency pulled by a build can touch
  anything the running user has access to: source files, home
  directory, network, the lot. Do not run as root. Do not run
  alongside credentials, SSH keys, or wallets you cannot afford to
  lose.
- **Assume everything is sent to Anthropic.** Every agent invocation
  ships task descriptions, file contents, and prior agent output to the
  Claude API. Do not point `cm` at anything you cannot legally,
  contractually, or ethically share.
- **Logs contain sensitive content.** `.cm/logs/<phase>.log` and
  `.cm/cm.log` capture full prompts and responses, including code
  excerpts the agents read and any secrets that happened to live in
  those files. Treat them like source code: review before publishing,
  do not commit to public repos.
- **No security review of dependencies or network code.** Subcommands
  like `cm update-pricing` perform HTTP fetches and parse third-party
  data; the dependency choices and parsing logic are LLM-generated and
  unaudited.
- **No warranty.** This is an experimental tool published as-is. If
  using it costs you money, time, data, or a production incident, that
  is on you.

If any of the above is a hard no for your environment, **don't run
`cm` yet** - wait for an independent audit, or run it only inside
throwaway sandboxes you can wipe.

## Features

- **Strict per-phase agent flow** - PLAN → IMPLEM → BUILD → REVIEW, with a
  FIX → re-REVIEW loop until approval or max cycles is reached
- **Agent context isolation** - no direct agent-to-agent communication;
  each agent is spawned with only the context it needs
- **Build verification** - runs configured build commands (e.g.
  `cargo build` + `cargo clippy`) after each implementation step
- **Full auditability** - per-phase logs (full prompts and responses for
  every agent invocation), persistent operational log, structured
  `log_records` in the source-of-truth JSON, and post-run reports
- **Crash recovery** - checkpoints with state restoration via `--continue`
- **Deterministic generation** - JSON is source of truth, markdown files
  are always regenerated
- **Config file support** - TOML configuration via `.cm/config.toml`
- **Signal handling** - graceful Ctrl+C shutdown with state preservation
- **Token usage reporting** - `cm token` shows usage and estimated cost,
  optionally broken down by project, day, week, or month

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- [Claude Code](https://github.com/anthropics/claude-code) installed and configured

### Build from source

```bash
git clone https://github.com/your-repo/cm.git
cd cm
cargo build --release
```

### Install to PATH

```bash
just install            # builds and copies to /usr/bin/cm (uses sudo)
# or
cargo install --path .  # installs to ~/.cargo/bin
```

## Quick Start

The intended workflow has two halves: **you and Claude Code build the
structured plan together**, then **`cm` executes it autonomously**. A
typical session looks like this:

### 0. Install the wizards into your repo

```bash
cm init                 # writes .claude/commands/ and .cm/agents/ templates
```

You only do this once per repo (or after pulling new wizard versions).

### 1. Brainstorm a high-level plan in Claude Code's plan mode

Open Claude Code in the repo, drop into plan mode, describe what you want
to build, and let Claude produce a high-level plan. This is freeform: you
are still iterating on intent, not yet committing to a structure.

```bash
claude
> # describe the project / feature, refine in plan mode until the
> # high-level approach feels right
```

### 2. Run `/cm` to convert intent into a structured plan

```
> /cm
```

The wizard walks you through project name, goals, success criteria, phase
breakdown (one task per phase by default), build commands, and reference
implementations. It uses the high-level plan from step 1 as input.

### 3. Polish the structured plan

Review what `/cm` proposes. Edit `PLAN.md` directly, push back on phase
sizes, ask the wizard to split or merge phases, refine per-task plan files
under `.cm/plans/`. This is the last point at which it is cheap to change
direction; spend time here.

### 4. Run `/end` to finalize

```
> /end
```

`/end` regenerates the markdown views from your edits, validates the JSON,
and commits the structured plan. After this step, `tasks.json` and
`roadmap.json` are the source of truth.

### 5. Hand off to `cm` and let it run

```bash
cm                      # autonomous execution, end to end
```

`cm` now takes over. It picks the next runnable task, runs the full
PLAN -> IMPLEM -> BUILD -> REVIEW -> (FIX -> re-REVIEW) cycle on each
phase, and stops only when every phase is complete or `max_cycles` defers
one. Ctrl+C is safe at any point: state is preserved, resume with
`cm --continue`.

For incremental work later (a new feature, a bug fix), use `/feat` or
`/fix` instead of `/cm`: same structured-plan output, scoped to one new
phase rather than a whole project.

## Usage

```bash
# Subcommands
cm init                 # install Claude Code wizards into the current repo
cm token                # show token usage + estimated cost for this project
cm update-pricing       # refresh [pricing.*] in .cm/config.toml from LiteLLM

# Execution modes
cm                      # run all tasks until completion
cm --continue           # resume from interrupted state
cm --step               # execute one task, then pause
cm --dry-run            # preview without executing

# Status & maintenance
cm --status             # show progress summary
cm --validate           # validate tasks.json schema
cm --sanity-check       # deep cross-reference validation
cm --regenerate         # regenerate MD files from JSON
cm --prune              # trim phase log entries older than 24h
cm --reset <PHASE_ID>   # reset a phase to pending

# Options
cm -v, --verbose        # debug logging
cm --config FILE        # custom config file (default: .cm/config.toml)
cm --state FILE         # custom state file (default: .cm/tasks.json)
cm --model {sonnet|opus}
cm --max-cycles N       # cap fix/review cycles per task before deferring
cm --working-dir DIR    # working directory for build verification
```

## Wizards installed by `cm init`

| Wizard    | Purpose                                                                  |
| --------- | ------------------------------------------------------------------------ |
| `/cm`     | Interactive wizard to set up a new cm project                            |
| `/feat`   | Add a new feature (becomes a phase) to an existing cm project            |
| `/fix`    | Add a bug-fix task to an existing cm project                             |
| `/split`  | Split a phase that's grown too large into smaller phases                 |
| `/expand` | Expand the active plan with more detail                                  |
| `/clone`  | Generate a source-code specification from existing code                  |
| `/end`    | Finalize a `/feat` or `/fix` session by saving changes to planning files |
| `/run`    | Continue working on this cm-managed project                              |

## Token usage reporting

`cm token` walks Claude Code's session JSONL files (`~/.claude/projects/`) and
prints token consumption with an estimated USD cost. Pricing comes from
`.cm/config.toml`'s `[pricing.*]` table when present, with hardcoded
current-generation defaults as a fallback.

```bash
cm token                # current project, per-model totals
cm token --global       # all projects aggregated
cm token --breakdown    # per-project totals, sorted by cost descending
cm token --daily        # current project, broken down by date
cm token --weekly       # by ISO week
cm token --monthly      # by month
cm update-pricing       # refresh [pricing.*] from LiteLLM's public dataset
```

`--breakdown` is mutually exclusive with the time-bucket flags. All reports
include a `Since YYYY-MM-DD` line showing the earliest recorded usage.

## Configuration

Create `.cm/config.toml` for persistent settings:

```toml
model = "claude-sonnet-4-5-20250929"
timeout_secs = 300
max_cycles = 5
build_commands = ["cargo build", "cargo clippy"]
```

See [CONFIG.md](CONFIG.md) for all options.

**Precedence:** CLI flags > config file > defaults

## Project Structure

```
your-project/
├── .cm/
│   ├── tasks.json         # task definitions & state (source of truth)
│   ├── roadmap.json       # roadmap state (source of truth)
│   ├── config.toml        # configuration
│   ├── PLAN.md            # high-level plan
│   ├── ROADMAP.md         # detailed checklist (generated)
│   ├── TASKS.md           # task status (generated)
│   ├── plans/             # per-task plan files
│   ├── agents/            # IMPLEMENTER/REVIEWER/PLANNER/FIX templates
│   ├── logs/              # per-phase logs
│   ├── checkpoints/       # recovery snapshots
│   └── cm.log             # persistent operational log
├── .claude/
│   └── commands/
│       ├── cm.md
│       ├── feat.md
│       ├── fix.md
│       ├── end.md
│       ├── run.md
│       ├── split.md
│       ├── expand.md
│       └── clone.md
├── src/
└── Cargo.toml
```

## How It Works

For every phase, `cm` runs a strict, agent-isolated cycle. Each agent is
spawned with only the context it needs (its task description, the relevant
files, the code-style excerpt, and - for FIX/re-REVIEW - the prior REVIEW's
findings). Agents never share context directly; the manager mediates every
hand-off.

```
Happy path, per phase:
    PLAN → IMPLEM → BUILD → REVIEW → approved → next phase

When REVIEW finds issues:
    FIX → BUILD → REVIEW → (repeat up to max_cycles, then defer the phase)
```

1. **Task selection** - manager picks the next runnable task respecting
   `depends_on` ordering.
2. **PLAN** - the PLANNER agent reviews the per-task plan in
   `.cm/plans/<task-id>.md`. If the plan is already detailed enough it
   returns `null`; otherwise it expands the plan in place.
3. **IMPLEM** - the IMPLEMENTER agent applies the plan to the codebase
   with task-only context.
4. **Build verification** - runs every command in `build_commands` (e.g.
   `cargo build`, `cargo clippy`). A failure short-circuits to a
   build-fix injection rather than progressing to REVIEW.
5. **REVIEW** - the REVIEWER agent inspects the diff with fresh context.
   It either approves the change or produces a list of issues.
6. **FIX → re-REVIEW loop** - on a non-approving REVIEW, a FIX task is
   spawned with the prior issues as input, the build is re-verified, and
   REVIEW runs again. The loop repeats up to `max_cycles` times (default
   5). If still not approved, the phase is marked `deferred` and the
   manager moves on; the user can re-run later with `cm --reset <phase>`.
7. **Phase completion** - after approval and a green build, the manager
   updates `tasks.json` / `roadmap.json`, regenerates `TASKS.md` and
   `ROADMAP.md`, and proceeds.
8. **Post-run review** - once all phases are done, an end-to-end review
   pass produces a report under `.cm/reports/`.

Cycle counts, status transitions, and timeouts are all configurable via
`.cm/config.toml` (see [CONFIG.md](CONFIG.md)).

## Auditability

Every step of the flow above leaves a durable record. Nothing the manager
or its agents do is invisible.

- **`.cm/logs/<phase-id>.log`**  
  Per-phase log: full prompts and full responses for **every** PLAN /
  IMPLEM / REVIEW / FIX agent invocation in that phase, timestamped to
  the millisecond.

- **`.cm/cm.log`**  
  Persistent operational log across all runs (manager state transitions,
  build outcomes, signals). Trim with `cm --prune` (drops entries older
  than 24h).

- **`tasks.json` → `log_records`**  
  Structured records (timestamps, task IDs, agent kinds, outcomes)
  embedded inside the source-of-truth JSON. Survives crashes and
  re-runs.

- **`tasks.json` → `attempts`**  
  Per-task execution history: which cycles ran, what each produced,
  what the build said.

- **`.cm/checkpoints/`**  
  Snapshots of `tasks.json` taken before risky transitions; used by
  `cm --continue` to resume a crashed run.

- **`.cm/reports/`**  
  Post-run review reports written after the final phase completes.

- **`cm --status`**  
  Live progress summary derived from `tasks.json`.

- **`cm --validate` / `cm --sanity-check`**  
  Schema and cross-reference validation of the JSON state.

Because JSON is the source of truth and the markdown files are regenerated
from it, the audit trail is the same whether you read it through `cm` or
inspect the files directly.

## JSON as source of truth

`cm` uses JSON files as the source of truth:
- `tasks.json` - task state and log records
- `roadmap.json` - roadmap structure

Markdown files (ROADMAP.md, TASKS.md) are **generated** from JSON:

```bash
cm --regenerate         # regenerate all MD files from JSON
```

## License

MIT
