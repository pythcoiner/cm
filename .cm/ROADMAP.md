# Claude Code Manager (cm) - Roadmap

This document tracks implementation progress. Check off items as they are completed.

## Phase 0: Project Setup

Status: **Not Started** (0/0)


---

## Phase 0.5: cm init + Skills

Status: **Not Started** (0/0)


---

## Phase 1: State Types

Status: **Not Started** (0/0)


---

## Phase 2: Agent Module

Status: **Not Started** (0/0)


---

## Phase 3: Build Module

Status: **Not Started** (0/0)


---

## Phase 4: Log Module

Status: **Not Started** (0/0)


---

## Phase 5: Manager Module

Status: **Not Started** (0/0)


---

## Phase 6: Recovery Module

Status: **Not Started** (0/0)


---

## Phase 7: CLI Module

Status: **Not Started** (0/0)


---

## Phase 8: Skills Module

Status: **Not Started** (0/0)


---

## Phase 9: Integration Tests

Status: **Not Started** (0/0)


---

## Phase 10: TUI Module

Status: **Not Started** (0/0)


---

## Phase 11: Polish

Status: **Not Started** (0/0)


---

## Phase 12: Configuration

Status: **Not Started** (0/0)


---

## Phase 13: TUI Integration

Status: **Not Started** (0/0)


---

## Phase 14: Signal Handling

Status: **Not Started** (0/0)


---

## Phase 15: JSON Source of Truth

Status: **Not Started** (0/0)


---

## Phase 16: Sanity Check Workflow

Status: **Not Started** (0/0)


---

## Phase 18: Regenerate MD After Skills

Status: **Not Started** (0/0)


---

## Phase 23: Detailed File Logging

Status: **Not Started** (0/0)


---

## Phase 26: Skills to Commands Migration

Status: **Not Started** (0/0)


---

## Phase 27: Add /run Command

Status: **Complete** (1/1)

- [x] Add /run command for cm orchestration
  - [x] Create assets/run.md with orchestration prompt
  - [x] Add RUN_COMMAND constant to src/command.rs
  - [x] Update src/cli/init.rs with run command

---

## Phase 28: Remove LOG.md Generation

Status: **Complete** (1/1)

- [x] Remove LOG.md generation entirely
  - [x] Delete src/generate/log_md.rs
  - [x] Update src/generate/mod.rs
  - [x] Update src/cli/mod.rs regenerate command
  - [x] Update tests/integration.rs
  - [x] Update documentation (CLAUDE.md, README.md, assets/*.md)
  - [x] Delete .cm/LOG.md

---

## Phase 29: Add --reset Flag

Status: **Complete** (1/1)

- [x] Add --reset flag to reset a phase
  - [x] Add reset: Option<String> to Cli struct
  - [x] Add dispatch in run() function
  - [x] Implement execute_reset() function
  - [x] Update CLAUDE.md documentation

---

## Phase 30: Add /cm Prerequisites Check

Status: **Not Started** (0/1)

- [ ] Add Step 0 prerequisites check to /cm wizard
  - [ ] Add Step 0 section to assets/cm.md
  - [ ] Check for .cm/agents/ directory
  - [ ] Provide guidance to run cm init

---

## Phase 31: Add .cm/*.log to .gitignore

Status: **Not Started** (0/1)

- [ ] Add .cm/*.log to root .gitignore during cm init
  - [ ] Add ensure_gitignore_entry() function
  - [ ] Call from execute_init_in_dir()
  - [ ] Add tests for new and existing .gitignore

---

## Summary

| Phase | Status | Progress |
|-------|--------|----------|
| Phase 0: Project Setup | Not Started | 0/0 |
| Phase 0.5: cm init + Skills | Not Started | 0/0 |
| Phase 1: State Types | Not Started | 0/0 |
| Phase 2: Agent Module | Not Started | 0/0 |
| Phase 3: Build Module | Not Started | 0/0 |
| Phase 4: Log Module | Not Started | 0/0 |
| Phase 5: Manager Module | Not Started | 0/0 |
| Phase 6: Recovery Module | Not Started | 0/0 |
| Phase 7: CLI Module | Not Started | 0/0 |
| Phase 8: Skills Module | Not Started | 0/0 |
| Phase 9: Integration Tests | Not Started | 0/0 |
| Phase 10: TUI Module | Not Started | 0/0 |
| Phase 11: Polish | Not Started | 0/0 |
| Phase 12: Configuration | Not Started | 0/0 |
| Phase 13: TUI Integration | Not Started | 0/0 |
| Phase 14: Signal Handling | Not Started | 0/0 |
| Phase 15: JSON Source of Truth | Not Started | 0/0 |
| Phase 16: Sanity Check Workflow | Not Started | 0/0 |
| Phase 18: Regenerate MD After Skills | Not Started | 0/0 |
| Phase 23: Detailed File Logging | Not Started | 0/0 |
| Phase 26: Skills to Commands Migration | Not Started | 0/0 |
| Phase 27: Add /run Command | Complete | 4/4 |
| Phase 28: Remove LOG.md Generation | Complete | 7/7 |
| Phase 29: Add --reset Flag | Complete | 5/5 |
| Phase 30: Add /cm Prerequisites Check | Not Started | 0/4 |
| Phase 31: Add .cm/*.log to .gitignore | Not Started | 0/4 |
| **Total** | | **16/24** |
