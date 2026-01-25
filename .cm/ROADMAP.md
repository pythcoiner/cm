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

Status: **Not Started** (0/1)

- [ ] Add /run command for cm orchestration
  - [ ] Create assets/run.md with orchestration prompt
  - [ ] Add RUN_COMMAND constant to src/command.rs
  - [ ] Update src/cli/init.rs with run command

---

## Phase 28: Remove LOG.md Generation

Status: **Not Started** (0/1)

- [ ] Remove LOG.md generation entirely
  - [ ] Delete src/generate/log_md.rs
  - [ ] Update src/generate/mod.rs
  - [ ] Update src/cli/mod.rs regenerate command
  - [ ] Update tests/integration.rs
  - [ ] Update documentation (CLAUDE.md, README.md, assets/*.md)
  - [ ] Delete .cm/LOG.md

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
| Phase 27: Add /run Command | Not Started | 0/4 |
| Phase 28: Remove LOG.md Generation | Not Started | 0/7 |
| **Total** | | **0/11** |
