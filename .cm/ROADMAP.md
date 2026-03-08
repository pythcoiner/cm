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

Status: **Complete** (1/1)

- [x] Add Step 0 prerequisites check to /cm wizard
  - [x] Add Step 0 section to assets/cm.md
  - [x] Check for .cm/agents/ directory
  - [x] Provide guidance to run cm init

---

## Phase 31: Add .cm/logs/ to .gitignore

Status: **Complete** (1/1)

- [x] Add .cm/logs/ to root .gitignore during cm init
  - [x] Add ensure_gitignore_entry() function
  - [x] Call from execute_init_in_dir()
  - [x] Add tests for new and existing .gitignore

---

## Phase 32: Add Phase Selection by Number

Status: **Complete** (1/1)

- [x] Add [p]hase <#...> option to run specific phases by number
  - [x] Add Phases(Vec<String>) variant to TaskSelection enum
  - [x] Update prompt text to show [p]hase <#...> option
  - [x] Add parsing for 'p 3 5 7' input format
  - [x] Add run_specific_phases() method
  - [x] Handle Phases variant in run_interactive()

---

## Phase 33: Add PLAN Agent Before IMPLEM

Status: **Complete** (1/1)

- [x] Add PLAN agent step before IMPLEM with conditional detailed planning
  - [x] Create PLANNER.md template in assets/templates/
  - [x] Add PLANNER_TEMPLATE to src/command.rs
  - [x] Add PlanAgentResponse struct to src/agent/response.rs
  - [x] Add build_phase_plan_prompt() to src/agent/prompt.rs
  - [x] Modify execute_phase() to run PLAN agent before IMPLEM
  - [x] Log PLAN agent prompt/response same as other agents

---

## Phase 34: Add /split Command + Extend /end

Status: **Complete** (2/2)

- [x] Add /split command to refine PLAN.md into detailed phases
  - [x] Create assets/split.md with split wizard
  - [x] Add SPLIT_COMMAND constant to src/command.rs
  - [x] Update src/cli/init.rs to deploy split.md
- [x] Extend /end to support /cm session finalization
  - [x] Add session type detection to assets/end.md
  - [x] Add PLAN.md parsing and JSON generation flow
  - [x] Generate tasks.json and roadmap.json from PLAN.md

---

## Phase 35: Remove TUI Module

Status: **Complete** (1/1)

- [x] Remove TUI and make daemon mode default
  - [x] Delete src/tui/ directory (mod.rs, layout.rs, widgets.rs)
  - [x] Remove pub mod tui from src/lib.rs
  - [x] Remove ratatui and crossterm from Cargo.toml
  - [x] Remove --daemon flag and TUI imports from src/cli/mod.rs
  - [x] Remove TUI channel fields and run_with_channels() from src/manager/mod.rs
  - [x] Update CLAUDE.md and .cm/PLAN.md documentation

---

## Phase 36: Remove Unused MANAGER.md

Status: **Complete** (1/1)

- [x] Remove unused MANAGER.md template and related code
  - [x] Delete assets/templates/MANAGER.md
  - [x] Remove MANAGER_TEMPLATE from src/command.rs
  - [x] Remove agents/MANAGER.md from TEMPLATES array in src/cli/init.rs
  - [x] Remove build_manager_prompt() from src/agent/prompt.rs
  - [x] Update assets/cm.md documentation

---

## Phase 37: Add Phase Range Selection

Status: **Complete** (1/1)

- [x] Add range-based phase selection (e.g., p 3-6)
  - [x] Modify parsing in prompt_task_selection() to handle ranges
  - [x] Use flat_map to expand ranges like 3-6 into phase-3, phase-4, phase-5, phase-6
  - [x] Support mixed input like p 1 3-5 8

---

## Phase 38: Add Model Selection Flag

Status: **Complete** (1/1)

- [x] Add --model flag with sonnet/opus shorthand
  - [x] Add ModelChoice enum with ValueEnum derive
  - [x] Change model field to Option<ModelChoice> in Cli struct
  - [x] Map enum to strings in build_manager_config()
  - [x] Update default model to 'sonnet' in ManagerConfig

---

## Phase 39: Fix Agent Spawn Failures

Status: **Not Started** (0/3)

- [ ] Pass agent prompt via stdin instead of -p CLI arg
- [ ] Revert task status on spawn failure and stop on phase error
- [ ] Review spawn fix and error handling

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
| Phase 30: Add /cm Prerequisites Check | Complete | 4/4 |
| Phase 31: Add .cm/logs/ to .gitignore | Complete | 4/4 |
| Phase 32: Add Phase Selection by Number | Complete | 6/6 |
| Phase 33: Add PLAN Agent Before IMPLEM | Complete | 7/7 |
| Phase 34: Add /split Command + Extend /end | Complete | 8/8 |
| Phase 35: Remove TUI Module | Complete | 7/7 |
| Phase 36: Remove Unused MANAGER.md | Complete | 6/6 |
| Phase 37: Add Phase Range Selection | Complete | 4/4 |
| Phase 38: Add Model Selection Flag | Complete | 5/5 |
| Phase 39: Fix Agent Spawn Failures | Not Started | 0/3 |
| **Total** | | **67/70** |
