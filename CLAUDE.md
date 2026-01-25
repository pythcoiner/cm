# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**cm (Claude Code Manager)** is a Rust CLI tool that automates multi-phase software development by orchestrating Claude AI agents with strict context isolation. Agents never communicate directly; the manager mediates all interactions through an IMPLEM -> VERIFY -> FIX -> RE-VERIFY workflow.

## Build & Development Commands

```bash
cargo build --release       # Build release binary (target/release/cm)
cargo test                  # Run integration tests (tests/integration.rs)
cargo clippy                # Lint (must pass clean)
just build                  # Same as cargo build --release
just install                # Build + install to /usr/bin/cm (requires sudo)
```

All three checks (build, clippy, test) must pass before committing.

## Architecture

### Core Flow

The manager selects the next runnable task, spawns an IMPLEM agent with isolated context, runs build verification (`cargo build` + `clippy`), spawns a REVIEW agent, and either marks the task complete or enters a FIX/re-review cycle (max 5 cycles before deferring).

### Module Layout

- **`src/main.rs`** - Entry point, delegates to `cli::run`
- **`src/cli/`** - Clap-based argument parsing (`mod.rs`) and `cm init` subcommand (`init.rs`)
- **`src/command.rs`** - Embedded `.md` files for `/cm`, `/feat`, `/fix`, `/end` skills
- **`src/state/`** - State management with `tasks.json` as source of truth. Includes task/phase types (`tasks.rs`), log records (`log_record.rs`), roadmap state (`roadmap.rs`), and JSON schema validation (`validate.rs`)
- **`src/manager/`** - Main orchestration loop (`mod.rs`), manager state machine (`state.rs`), and crash recovery with checkpointing (`recovery.rs`)
- **`src/agent/`** - Agent spawning (`mod.rs`), prompt building with context isolation (`prompt.rs`), and JSON response parsing (`response.rs`)
- **`src/build/`** - Build verification (`mod.rs`), cargo runner (`cargo.rs`), and git operations (`git.rs`)
- **`src/log/`** - Per-phase detailed logging (`mod.rs`) and persistent `cm.log` with rotation (`file_logger.rs`)
- **`src/generate/`** - Deterministic markdown generation from JSON: `roadmap_md.rs` (ROADMAP.md) and `tasks_md.rs` (TASKS.md)
- **`src/tui/`** - Terminal UI with ratatui/crossterm: split layout (`layout.rs`), widgets (`widgets.rs`), and stream management (`mod.rs`)
- **`src/config/`** - TOML configuration loading from `.cm/config.toml`

### Key Design Decisions

- **JSON is source of truth** - `tasks.json` and `roadmap.json` are definitive; markdown files (ROADMAP.md, TASKS.md) are always regenerated from JSON, never edited directly
- **Agent context isolation** - Each agent receives only its task description, relevant files, code style excerpt, and prior review issues (if applicable). No global context or cross-task information
- **Threading model** - Uses `std::thread` (not async). Main thread runs TUI, background thread runs Manager. Communication via `std::sync::mpsc` channels and `Arc<AtomicBool>` for shutdown signaling
- **Error handling** - All error types use `thiserror` derive macro with typed enums. Never use `anyhow` or string errors
- **Config precedence** - CLI args > `.cm/config.toml` > defaults

### `.cm/` Directory

Project state lives in `.cm/`:
- `tasks.json` - Phases, tasks, attempts, log records (source of truth)
- `roadmap.json` - Roadmap structure for `/cm` skill
- `config.toml` - Optional configuration
- `PLAN.md`, `ROADMAP.md`, `TASKS.md` - Human-readable files (markdown generated from JSON)
- `cm.log` - Persistent operational debug log

## Code Conventions

- Serde enums use `#[serde(rename_all = "snake_case")]`; optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`
- Config structs use fluent builder pattern (e.g., `ManagerConfig::new(path).model(...).timeout(...)`)
- One responsibility per module; test modules at end of file with `#[cfg(test)]`
- Commit messages follow: `cm: Phase N - description`

## CLI Modes

```
cm                  # Run all tasks with TUI
cm --step           # Run one task only
cm --continue       # Resume from crash
cm --status         # Show progress
cm --validate       # Validate tasks.json schema
cm --sanity-check   # Deep JSON validation
cm --regenerate     # Regenerate markdown from JSON
cm --dry-run        # Preview without executing
cm --prune          # Trim cm.log to last 24 hours
cm --reset <ID>     # Reset phase to pending
cm init             # Create .claude/commands/ files
```
