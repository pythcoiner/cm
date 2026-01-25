# Claude Code Manager (cm)

> Automated agent coordination for software development - orchestrates Claude AI agents to execute multi-phase development tasks autonomously.

## Overview

Claude Code Manager (cm) is a Rust CLI tool that automates coordination between Claude agents for software development tasks. The manager mediates all agent communication - agents never interact directly with each other, ensuring context isolation and independent verification.

The tool follows a strict IMPLEM → REVIEW → FIX cycle where implementation agents receive only task-specific context, review agents verify the work with fresh context, and fix agents address any issues found. This prevents context pollution and ensures each agent works independently.

cm supports crash recovery through checkpointing, real-time monitoring via a terminal UI, and deterministic markdown generation where JSON files serve as the source of truth.

## Goals

- Orchestrate multi-phase development tasks with dependent tasks
- Ensure agent context isolation (no direct agent-to-agent communication)
- Provide automatic build verification (cargo build + clippy)
- Enable crash recovery with checkpointing and state restoration
- Offer real-time TUI for monitoring execution
- Support TOML configuration files
- Handle graceful Ctrl+C shutdown with state preservation
- Maintain JSON as source of truth with deterministic MD generation

## Success Criteria

- [x] Can define phases with dependent tasks in tasks.json
- [x] Agents receive only task-specific context, never global knowledge
- [x] Build verification runs after each implementation
- [x] State is checkpointed before mutations for recovery
- [x] TUI shows real-time progress with keyboard controls
- [x] Configuration can be set via CLI, config file, or defaults
- [x] Ctrl+C saves state and exits gracefully
- [x] MD files are regenerated deterministically from JSON

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    cm (Rust binary)                     │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   State     │  │   Agent     │  │   Build         │ │
│  │   Manager   │  │   Spawner   │  │   Verifier      │ │
│  │ tasks.json  │  │ claude -p   │  │ cargo/git       │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
└──────────────────────────┬──────────────────────────────┘
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
│ IMPLEM Agent│     │REVIEW Agent │     │  FIX Agent  │
│ Task ctx    │     │ Same ctx    │     │ Ctx+issues  │
│ only        │     │ as IMPLEM   │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Components

1. **State Manager** - Handles tasks.json loading, saving, and state transitions
2. **Agent Spawner** - Spawns claude CLI processes with isolated context
3. **Build Verifier** - Runs cargo build/clippy/test for verification
4. **Log Manager** - Creates LogRecords for append-only audit trail
5. **Recovery Manager** - Handles checkpointing and crash recovery
6. **TUI** - Terminal UI with ratatui for real-time monitoring

### Data Flow

1. Manager loads state from tasks.json
2. Selects next runnable task (respects dependencies)
3. Builds prompt with task-only context
4. Spawns agent via claude CLI
5. Parses response and runs build verification
6. Creates review task or marks complete
7. Saves state and regenerates MD files

## Modules

### State Module (`src/state/`)

**Purpose:** Define and manage all state types

**Key files:**
- `mod.rs` - State loading/saving, StateError
- `tasks.rs` - TasksState, Task, Phase, TaskStatus types
- `log_record.rs` - LogRecord, LogAction, LogData types
- `roadmap.rs` - RoadmapState, RoadmapPhase, RoadmapItem types

**Dependencies:** serde, chrono, uuid

### Agent Module (`src/agent/`)

**Purpose:** Spawn and manage Claude agent processes

**Key files:**
- `mod.rs` - AgentSpawner, AgentHandle, AgentError
- `prompt.rs` - PromptBuilder for IMPLEM/REVIEW/FIX prompts
- `response.rs` - ResponseParser for JSON output

**Dependencies:** std::process, std::thread

### Build Module (`src/build/`)

**Purpose:** Run cargo and git commands for verification

**Key files:**
- `mod.rs` - BuildVerifier, BuildError
- `cargo.rs` - CargoRunner for build/clippy/test
- `git.rs` - GitRunner for status/add/commit

**Dependencies:** std::process

### Manager Module (`src/manager/`)

**Purpose:** Main orchestration loop

**Key files:**
- `mod.rs` - Manager, ManagerConfig, orchestration loop
- `state.rs` - ManagerState enum
- `recovery.rs` - RecoveryManager, ShutdownHandler

**Dependencies:** state, agent, build, log modules

### TUI Module (`src/tui/`)

**Purpose:** Terminal UI for monitoring

**Key files:**
- `mod.rs` - App, run_tui, terminal setup
- `layout.rs` - Split view layout
- `widgets.rs` - TaskList, Stream, Controls widgets

**Dependencies:** ratatui, crossterm

### Generate Module (`src/generate/`)

**Purpose:** Deterministic markdown generation

**Key files:**
- `mod.rs` - GenerateError, regenerate functions
- `tasks_md.rs` - Generate TASKS.md from TasksState
- `roadmap_md.rs` - Generate ROADMAP.md from RoadmapState

**Dependencies:** state module

## Phases

### Phase 0: Project Setup

**Goal:** Initialize project structure with Cargo.toml and module stubs

**Deliverables:**
- Cargo.toml with all dependencies
- src/lib.rs with module declarations
- src/main.rs with CLI skeleton

### Phase 0.5: cm init + Skills

**Goal:** Enable users to install cm skills into any project

**Deliverables:**
- assets/{cm,feat,fix}.md skill files
- src/skill.rs with include_str! embedding
- src/cli/init.rs with init command

### Phase 1-4: Core Types

**Goal:** Implement state, agent, build, and log modules

**Deliverables:**
- All type definitions with serde
- Agent spawning and response parsing
- Build verification with cargo
- Append-only logging

### Phase 5-6: Manager and Recovery

**Goal:** Main orchestration loop with crash recovery

**Deliverables:**
- Manager with IMPLEM/REVIEW/FIX flows
- Checkpointing before mutations
- Graceful shutdown handling

### Phase 7-8: CLI and Skills

**Goal:** Complete CLI and Claude Code skill

**Deliverables:**
- All CLI flags and commands
- /cm interactive wizard skill

### Phase 9-11: Testing and Polish

**Goal:** Integration tests and polish features

**Deliverables:**
- 27 integration tests
- Terminal UI with ratatui
- --dry-run and --verbose modes

### Phase 12-15: Configuration and Generation

**Goal:** Config file support and deterministic generation

**Deliverables:**
- TOML configuration
- TUI wired into CLI
- Signal handling
- JSON as source of truth

### Phase 16: Sanity Check Workflow

**Goal:** Validate JSON files and improve /cm skill workflow

**Deliverables:**
- `--sanity-check` CLI flag for JSON validation
- Validation module (src/state/validate.rs)
- Updated /cm skill with post-generation workflow
- Unit tests for all validation types

**Validation checks:**
- Valid JSON syntax
- Schema compliance (required fields, correct types)
- Cross-references (roadmap_item_id ↔ roadmap.json, linked_task_ids ↔ tasks.json)
- Duplicate ID detection
- Orphaned reference detection

**Skill workflow additions:**
1. After generating JSON files, run `cm --sanity-check`
2. If validation fails, iterate until JSON is valid
3. Ask user if .cm/ should be added to .gitignore (default: no)
4. Auto-generate commit message, ask user for confirmation
5. Commit changes

### Phase 18: Regenerate MD After Skills

**Goal:** Ensure markdown files stay in sync when using /feat or /fix skills

**Deliverables:**
- Updated feat.md with `cm --regenerate` step after file modifications
- Updated fix.md with `cm --regenerate` step after file modifications

**Workflow:**
After skills modify JSON files (tasks.json, roadmap.json), they must run `cm --regenerate` to update the markdown documentation (ROADMAP.md, TASKS.md) before proceeding to completion.

### Phase 23: Detailed File Logging

**Goal:** Add persistent operational logging to `.cm/cm.log` for debugging and auditing

**Deliverables:**
- `src/log/file_logger.rs` - Thread-safe FileLogger with log levels
- `--prune` CLI flag to trim log to last 24 hours
- FileLogger integrated into Manager for comprehensive event logging

**Log Format:**
```
[2026-01-24 10:30:45.123] [INFO] [manager] Manager starting with model: claude-sonnet-4-5-20250929
[2026-01-24 10:30:45.200] [DEBUG] [state] State loaded: 5 phases, 23 tasks
[2026-01-24 10:30:45.250] [INFO] [task] Task selected: phase-1.task-1
```

**Events Logged:**
- Manager lifecycle (startup, shutdown, config)
- State operations (load, save)
- Task selection, completion, deferral
- Agent spawn/complete with duration
- Build verification results
- Signal handling (Ctrl+C)

**Log Levels:**
- DEBUG: State saves, prompt previews, blocked task details
- INFO: Major milestones (task start/complete, agent spawn)
- WARN: Build failures, task deferrals, shutdown signals
- ERROR: Fatal errors

### Phase 26: Skills to Commands Migration

**Goal:** Convert `/cm`, `/feat`, `/fix`, `/end` from Claude Code skills to commands

**Deliverables:**
- Asset files without YAML front matter, "skill" → "command" in text
- `src/command.rs` replacing `src/skill.rs` with renamed constants
- `src/cli/init.rs` installing to `.claude/commands/{name}.md` (flat files)
- Legacy `.claude/skills/` cleanup function
- Updated README.md with commands terminology

**Key changes:**
- Install path: `.claude/skills/{name}/SKILL.md` → `.claude/commands/{name}.md`
- Module: `crate::skill` → `crate::command`
- Constants: `*_SKILL` → `*_COMMAND`
- Struct: `SkillFile` → `CommandFile`

### Phase 35: Remove TUI Module

**Goal:** Remove Terminal UI and make daemon mode the default (and only) execution mode

**Deliverables:**
- Delete `src/tui/` directory (mod.rs, layout.rs, widgets.rs)
- Remove ratatui and crossterm dependencies from Cargo.toml
- Remove `--daemon` flag from CLI
- Simplify `execute_run()` to always use `run_interactive()`
- Remove TUI-related code from Manager (channels, `run_with_channels()`)
- Update documentation

**Rationale:**
- Simplifies codebase by ~950 lines
- Removes terminal UI library dependencies
- Interactive stdin/stdout prompts provide sufficient user interaction
- Daemon mode already fully implemented via `run_interactive()`

### Phase 36: Remove Unused MANAGER.md

**Goal:** Remove dead code - the MANAGER.md template and `build_manager_prompt()` function that are never used

**Deliverables:**
- Delete `assets/templates/MANAGER.md`
- Remove `MANAGER_TEMPLATE` constant from `src/command.rs`
- Remove template from `TEMPLATES` array in `src/cli/init.rs`
- Remove `build_manager_prompt()` function from `src/agent/prompt.rs`
- Update `assets/cm.md` documentation

**Rationale:**
- `build_manager_prompt()` is defined but never called in production code
- Actual orchestration is handled by Rust `Manager` struct, not an AI agent
- Dead code removal improves maintainability

### Phase 37: Add Phase Range Selection

**Goal:** Enhance the `p` command to support range syntax like `p 3-6`

**Deliverables:**
- Modify parsing in `prompt_task_selection()` to detect range syntax
- Expand ranges like `3-6` into individual phase IDs (phase-3, phase-4, phase-5, phase-6)
- Support mixed input like `p 1 3-5 8`

**Implementation:**
- Replace `.map()` with `.flat_map()` in phase parsing
- Check each token for `-` to detect ranges
- Parse start/end and generate inclusive range of phase IDs

### Phase 38: Add Model Selection Flag

**Goal:** Add `--model` flag with `sonnet`/`opus` shorthand values

**Claude CLI Reference:**
```
--model <model>  Model for the current session. Provide an alias for the
                 latest model (e.g. 'sonnet' or 'opus') or a model's full
                 name (e.g. 'claude-sonnet-4-5-20250929').
```

**Deliverables:**
- Add `ModelChoice` enum with `#[derive(ValueEnum)]` for clap validation
- Change `--model` flag from `Option<String>` to `Option<ModelChoice>`
- Map enum to simple strings (`sonnet`, `opus`) passed directly to claude CLI
- Update default model from hardcoded version to `sonnet`

**Rationale:**
- Simpler UX: `cm --model opus` instead of `cm --model claude-opus-4-5-20250929`
- No hardcoded model versions - claude CLI resolves aliases to latest
- Clap auto-validates input and shows allowed values in help

## Technical Decisions

### Error Handling

**Context:** Need consistent error handling across modules
**Decision:** Use thiserror for all error enums, never anyhow
**Rationale:** Type-safe errors with proper context propagation

### Persistence

**Context:** Need reliable state persistence for crash recovery
**Decision:** JSON files as source of truth, MD files generated
**Rationale:** JSON is machine-readable, MD is human-readable view

### Agent Spawning

**Context:** Need to run Claude agents with isolated context
**Decision:** Use `claude -p "prompt" --output-format json`
**Rationale:** CLI is stable interface, JSON output is parseable

### Threading

**Context:** Manager needs simple synchronous execution
**Decision:** Use std::thread and std::sync::mpsc, no async
**Rationale:** Simpler mental model, no need for async runtime

## Out of Scope

- Async/await (use std::thread instead)
- Direct agent-to-agent communication
- Global context leaking into agent prompts
- Tokio or other async runtimes

## References

- [clap documentation](https://docs.rs/clap)
- [serde documentation](https://serde.rs)
- [Claude CLI](https://github.com/anthropics/claude-code)
