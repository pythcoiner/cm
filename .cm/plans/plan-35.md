# Plan: Remove TUI and Make Daemon Mode Default

## Summary

Remove the Terminal UI (ratatui/crossterm) and `--daemon` flag entirely. The current daemon mode behavior becomes the only mode - interactive stdin/stdout prompts for task selection.

## Files to Delete

1. **`src/tui/mod.rs`** (639 lines) - Main TUI module
2. **`src/tui/layout.rs`** (72 lines) - Layout definitions
3. **`src/tui/widgets.rs`** (241 lines) - Widget rendering

## Files to Modify

### 1. `src/lib.rs`
- Remove: `pub mod tui;` declaration

### 2. `Cargo.toml`
- Remove dependencies:
  ```toml
  ratatui = "0.26"
  crossterm = "0.27"
  ```

### 3. `src/cli/mod.rs`
- Remove import: `use crate::tui::{self, ManagerEvent, TuiCommand};`
- Remove `--daemon` flag from `Cli` struct (lines 142-144)
- Simplify `execute_run()`: remove conditional, always call `manager.run_interactive()`
- Delete `execute_run_with_tui()` function entirely (lines 341-371)
- Update tests: remove daemon flag test

### 4. `src/manager/mod.rs`
- Remove import: `use crate::tui::{ManagerEvent, TuiCommand};`
- Remove fields from `Manager` struct: `event_tx`, `cmd_rx`
- Delete method: `run_with_channels()` (lines 358-483)
- Simplify `prompt_retry_cycles()`: remove TUI branch, keep only stdin/stdout logic

### 5. `CLAUDE.md`
- Remove TUI references from Architecture and Module Layout sections
- Update CLI modes (remove TUI mention)

### 6. `.cm/PLAN.md`
- Update Architecture diagram (remove TUI component)
- Update Components list
- Update Modules section

## Implementation Steps

1. Delete `src/tui/` directory (3 files)
2. Remove `pub mod tui;` from `src/lib.rs`
3. Remove ratatui/crossterm from `Cargo.toml`
4. Update `src/cli/mod.rs`:
   - Remove TUI imports
   - Remove `daemon` field from `Cli` struct
   - Simplify `execute_run()` to always use `run_interactive()`
   - Delete `execute_run_with_tui()` function
   - Remove daemon flag test
5. Update `src/manager/mod.rs`:
   - Remove TUI imports
   - Remove `event_tx`/`cmd_rx` fields from Manager struct
   - Delete `run_with_channels()` method
   - Simplify `prompt_retry_cycles()` to stdin-only
6. Update documentation (CLAUDE.md, .cm/PLAN.md)
7. Run `cargo build`, `cargo clippy`, `cargo test` to verify

## Behavior After Change

- `cm` runs in interactive mode with stdin/stdout prompts
- User sees: `[s]ingle / [a]ll / [p]hase <#...> / [q]uit:` prompt
- No terminal UI, no ratatui/crossterm dependencies
- Simpler codebase with ~950 fewer lines
