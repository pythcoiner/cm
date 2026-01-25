# Plan: Remove Unused MANAGER.md

## Summary

Remove the unused `MANAGER.md` template and related code. The `build_manager_prompt()` function is defined but never called - actual orchestration is done by Rust code in `Manager` struct.

## Files to Delete

1. **`assets/templates/MANAGER.md`** - The template file

## Files to Modify

### 1. `src/command.rs`
- Remove: `pub const MANAGER_TEMPLATE: &str = include_str!("../assets/templates/MANAGER.md");`

### 2. `src/cli/init.rs`
- Remove from `TEMPLATES` array: `TemplateFile { path: "agents/MANAGER.md", content: MANAGER_TEMPLATE }`
- Remove import of `MANAGER_TEMPLATE` if needed
- Update/remove related tests

### 3. `src/agent/prompt.rs`
- Remove: `build_manager_prompt()` function (lines 82-104)
- Remove: Related tests `test_build_manager_prompt*` (lines 1201-1219)

### 4. `assets/cm.md`
- Remove references to MANAGER.md creation in the `/cm` skill documentation

## Implementation Steps

1. Delete `assets/templates/MANAGER.md`
2. Remove `MANAGER_TEMPLATE` constant from `src/command.rs`
3. Remove template entry from `TEMPLATES` array in `src/cli/init.rs`
4. Remove `build_manager_prompt()` function from `src/agent/prompt.rs`
5. Remove related tests
6. Update `assets/cm.md` documentation
7. Run `cargo build`, `cargo clippy`, `cargo test` to verify

## Behavior After Change

- `cm init` no longer creates `.cm/agents/MANAGER.md`
- No functional change to actual orchestration (it was never used)
- Cleaner codebase with less dead code
