# cm - Tasks

This document shows phase plans and task status. Generated from tasks.json.

## phase-0: Project Setup

**Status:** Complete (1/1)

### Tasks

- [x] **phase-0.task-1**: Create Cargo.toml - Create Cargo.toml with all dependencies

---

## phase-0.5: cm init + Skills

**Status:** Complete (1/1)

### Tasks

- [x] **phase-0.5.task-1**: Create assets directory with skills - Create assets/ with cm.md, feat.md, fix.md

---

## phase-1: State Types

**Status:** Complete (1/1)

### Tasks

- [x] **phase-1.task-1**: Create state module - Create src/state/ with all types

---

## phase-2: Agent Module

**Status:** Complete (1/1)

### Tasks

- [x] **phase-2.task-1**: Create agent module - Create src/agent/ with spawner, prompt builder, response parser

---

## phase-3: Build Module

**Status:** Complete (1/1)

### Tasks

- [x] **phase-3.task-1**: Create build module - Create src/build/ with cargo and git runners

---

## phase-4: Log Module

**Status:** Complete (1/1)

### Tasks

- [x] **phase-4.task-1**: Create log module - Create src/log/ with LogManager

---

## phase-5: Manager Core

**Status:** Complete (1/1)

### Tasks

- [x] **phase-5.task-1**: Create manager module - Create src/manager/ with orchestration loop

---

## phase-6: Recovery

**Status:** Complete (1/1)

### Tasks

- [x] **phase-6.task-1**: Create recovery module - Create src/manager/recovery.rs with checkpointing

---

## phase-7: CLI Commands

**Status:** Complete (1/1)

### Tasks

- [x] **phase-7.task-1**: Create CLI module - Create src/cli/ with all commands

---

## phase-8: Claude Code Skill

**Status:** Complete (1/1)

### Tasks

- [x] **phase-8.task-1**: Create /cm skill - Create .claude/skills/cm.md with wizard

---

## phase-9: Integration Testing

**Status:** Complete (1/1)

### Tasks

- [x] **phase-9.task-1**: Create integration tests - Create tests/integration.rs

---

## phase-10: Terminal UI

**Status:** Complete (1/1)

### Tasks

- [x] **phase-10.task-1**: Create TUI module - Create src/tui/ with ratatui

---

## phase-11: Polish

**Status:** Complete (1/1)

### Tasks

- [x] **phase-11.task-1**: Add polish features - Add --dry-run and --verbose modes

---

## phase-12: Configuration File Support

**Status:** Complete (1/1)

### Tasks

- [x] **phase-12.task-1**: Add config support - Create src/config/ with TOML support

---

## phase-13: Wire TUI into CLI

**Status:** Complete (1/1)

### Tasks

- [x] **phase-13.task-1**: Wire TUI into CLI - Connect TUI to manager via channels

---

## phase-14: Enable Signal Handling

**Status:** Complete (1/1)

### Tasks

- [x] **phase-14.task-1**: Enable signal handling - Enable ctrlc and wire shutdown handler

---

## phase-15: Deterministic Markdown Generation

**Status:** Complete (1/1)

### Tasks

- [x] **phase-15.task-1**: Implement deterministic generation - Create src/generate/ with MD generators

---

## phase-15.5: Phase-Level Build Verification

**Status:** Complete (1/1)

### Tasks

- [x] **phase-15.5.fix-phase-level-verification**: Fix: Move build verification from per-task to per-phase execution - Move build verification from per-task to per-phase execution:

---

## phase-16: Sanity Check Workflow

**Status:** Complete (6/6)

### Tasks

- [x] **phase-16.task-1**: Create src/state/validate.rs - Create src/state/validate.rs with comprehensive validation:
- [x] **phase-16.task-2**: Add --sanity-check flag to CLI - Add --sanity-check CLI flag:
- [x] **phase-16.task-3**: Update assets/cm.md skill workflow - Update assets/cm.md to add post-generation workflow:
- [x] **phase-16.task-4**: Add unit tests for validation - Add unit tests for validation module:
- [x] **phase-16.task-5**: Add sanity check to assets/feat.md - Add sanity check validation step to assets/feat.md after file updates:
- [x] **phase-16.task-6**: Add sanity check to assets/fix.md - Add sanity check validation step to assets/fix.md after file updates:

---

## phase-17: Concise One-at-a-Time Questions in Skills

**Status:** Complete (4/4)

### Tasks

- [x] **phase-17.task-1**: Refactor assets/cm.md questions - Refactor cm.md to ask questions one at a time with concise context:
- [x] **phase-17.task-2**: Refactor assets/feat.md questions - Refactor feat.md to ask questions one at a time:
- [x] **phase-17.task-3**: Refactor assets/fix.md questions - Refactor fix.md to ask questions one at a time:
- [x] **phase-17.task-4**: Rebuild and verify skills - Rebuild cm and verify all skills work:

---

## phase-18: Regenerate MD After Skills

**Status:** Complete (2/2)

### Tasks

- [x] **phase-18.task-1**: Update assets/feat.md with regeneration step - Update assets/feat.md to run cm --regenerate after modifying project files:
- [x] **phase-18.task-2**: Update assets/fix.md with regeneration step - Update assets/fix.md to run cm --regenerate after modifying project files:

---

## phase-19: Skills No Auto-Implement

**Status:** Complete (3/3)

### Tasks

- [x] **phase-19.task-1**: Update assets/feat.md scope limitations - Update assets/feat.md to enforce scope limitations:
- [x] **phase-19.task-2**: Update assets/fix.md scope limitations - Update assets/fix.md to enforce scope limitations:
- [x] **phase-19.task-3**: Reinstall skills and verify - Reinstall skills and verify changes:

---

## phase-20: Simplify Skill Input

**Status:** Complete (3/3)

### Tasks

- [x] **phase-20.task-1**: Simplify assets/feat.md input - Simplify assets/feat.md Step 1 to only ask for a description:
- [x] **phase-20.task-2**: Simplify assets/fix.md input - Simplify assets/fix.md Step 1 to only ask for a description:
- [x] **phase-20.task-3**: Reinstall and verify simplified skills - Reinstall skills and verify simplified input:

---

## phase-21: Agent Templates & Project Documentation

**Status:** Complete (4/4)

### Tasks

- [x] **phase-21.task-1**: Create default template assets and update cm init - Create default template files and update cm init to install them:
- [x] **phase-21.task-2**: Load agent templates from disk at runtime - Update agent prompt building to load templates from disk:
- [x] **phase-21.task-3**: Update /cm skill to customize templates - Update assets/cm.md to optionally customize templates:
- [x] **phase-21.task-4**: Test and verify template system - Test the complete template system:

---

## phase-22: TUI Default Mode + Interactive Task Selection

**Status:** Complete (5/5)

### Tasks

- [x] **phase-22.task-1**: Update CLI: TUI default, add --daemon flag - Update CLI to make TUI the default mode:
- [x] **phase-22.task-2**: Add task selection prompt to manager - Add interactive task selection before running tasks:
- [x] **phase-22.task-3**: Implement TUI task selection widget - Implement task selection in TUI mode:
- [x] **phase-22.task-4**: Implement daemon mode stdin prompt - Implement task selection in daemon mode (stdin/stdout):
- [x] **phase-22.task-5**: Update tests for new default mode - Update tests for new behavior:

---

## phase-23: Detailed File Logging

**Status:** Complete (7/7)

### Tasks

- [x] **phase-23.task-1**: Create FileLogger module - Create src/log/file_logger.rs with thread-safe file logging:
- [x] **phase-23.task-2**: Add prune functionality to FileLogger - Add log pruning to FileLogger:
- [x] **phase-23.task-3**: Export FileLogger from log module - Export FileLogger from src/log/mod.rs:
- [x] **phase-23.task-4**: Add --prune CLI flag - Add --prune CLI flag to src/cli/mod.rs:
- [x] **phase-23.task-5**: Add FileLogger to Manager - Integrate FileLogger into Manager:
- [x] **phase-23.task-6**: Add logging calls throughout Manager - Add logging calls to Manager execution flow:
- [x] **phase-23.task-7**: Add unit tests for FileLogger - Add unit tests for FileLogger in src/log/file_logger.rs:

---

## phase-24: /end Skill

**Status:** Complete (6/6)

### Tasks

- [x] **phase-24.task-1**: Create assets/end.md skill - Create assets/end.md with the /end skill content.
- [x] **phase-24.task-2**: Register END_SKILL in src/skill.rs - Add the END_SKILL constant to src/skill.rs:
- [x] **phase-24.task-3**: Register end skill in src/cli/init.rs - Register the /end skill in src/cli/init.rs:
- [x] **phase-24.task-4**: Modify assets/feat.md to delegate saves to /end - Modify assets/feat.md to remove file-update steps and delegate to /end:
- [x] **phase-24.task-5**: Modify assets/fix.md to delegate saves to /end - Modify assets/fix.md to remove file-update steps and delegate to /end:
- [x] **phase-24.task-6**: Build, reinstall skills, and verify - Build and verify the /end skill:

---

## phase-25: Labeled Stderr Output

**Status:** Complete (3/3)

### Tasks

- [x] **phase-25.task-1**: Thread agent_label through AgentSpawner - Add an `agent_label: &str` parameter to `spawn()`, `spawn_with_continue()`, and `run_agent_thread()` in src/agent/mod.rs. Thread the label through to the eprint lines:
- [x] **phase-25.task-2**: Pass agent labels at spawn sites and add [CM] eprints - Update src/manager/mod.rs to pass agent labels at all spawn call sites and add [CM] eprint lines for orchestration events.
- [x] **phase-25.task-3**: Build and verify labeled output - Verify the labeled stderr output changes:

---

## phase-26: Skills to Commands Migration

**Status:** Complete (7/7)

### Tasks

- [x] **phase-26.task-1**: Remove YAML front matter and update skill->command text in assets - Update all four asset markdown files to remove YAML front matter and replace self-referential 'skill' with 'command'.
- [x] **phase-26.task-2**: Rename src/skill.rs to src/command.rs with updated constants - Rename the skill module to command module:
- [x] **phase-26.task-3**: Update src/cli/mod.rs help text - Update the Command enum doc comments in src/cli/mod.rs:
- [x] **phase-26.task-4**: Rewrite src/cli/init.rs for commands - Rewrite src/cli/init.rs to install commands instead of skills:
- [x] **phase-26.task-5**: Update README.md - Update README.md to reflect the skills-to-commands change:
- [x] **phase-26.task-6**: Delete legacy .claude/skills/ directory - Remove the legacy .claude/skills/ directory from this repository:
- [x] **phase-26.task-7**: Build and verify commands migration - Final verification of the skills-to-commands migration:

---

## phase-27: Per-Phase TRACE Logging

**Status:** Complete (6/6)

### Tasks

- [x] **phase-27.task-1**: Add TRACE level to LogLevel enum - Add TRACE log level to FileLogger:
- [x] **phase-27.task-2**: Create PhaseLogger struct - Create PhaseLogger for per-phase TRACE logging in a new file:
- [x] **phase-27.task-3**: Integrate PhaseLogger into Manager - Integrate PhaseLogger into Manager to log full prompts and responses:
- [x] **phase-27.task-4**: Update --prune for per-phase logs - Update --prune CLI flag to handle per-phase log files:
- [x] **phase-27.task-5**: Add unit tests for PhaseLogger - Add comprehensive unit tests for PhaseLogger:
- [x] **phase-27.task-6**: Build and verify per-phase logging - Final verification of per-phase TRACE logging:

---

## phase-28: FIX Cycle Retry & Resume

**Status:** Complete (1/1)

### Tasks

- [x] **phase-28.fix-retry-prompt**: Fix: Interactive retry prompt after max FIX cycles + resume at REVIEW on --continue - Fix the FIX cycle retry and recovery behavior:

---

## phase-29: Per-Phase Agent Spawning

**Status:** Complete (1/1)

### Tasks

- [x] **phase-29.task-1**: Implement per-phase agent spawning architecture - Change cm from spawning an agent per task to spawning an agent per phase:

---

## phase-30: Fix Roadmap Subitem Updates

**Status:** Pending (0/1)

### Tasks

- [ ] **phase-30.fix-roadmap-subitem-id**: Fix: Roadmap subitems not updated (missing ID field) - Fix roadmap subitem update bug by adding ID field to RoadmapSubItem:

---

## Summary

| Phase | Status | Tasks | Completed |
|-------|--------|-------|----------:|
| phase-0: Project Setup | Complete | 1 | 1 |
| phase-0.5: cm init + Skills | Complete | 1 | 1 |
| phase-1: State Types | Complete | 1 | 1 |
| phase-2: Agent Module | Complete | 1 | 1 |
| phase-3: Build Module | Complete | 1 | 1 |
| phase-4: Log Module | Complete | 1 | 1 |
| phase-5: Manager Core | Complete | 1 | 1 |
| phase-6: Recovery | Complete | 1 | 1 |
| phase-7: CLI Commands | Complete | 1 | 1 |
| phase-8: Claude Code Skill | Complete | 1 | 1 |
| phase-9: Integration Testing | Complete | 1 | 1 |
| phase-10: Terminal UI | Complete | 1 | 1 |
| phase-11: Polish | Complete | 1 | 1 |
| phase-12: Configuration File Support | Complete | 1 | 1 |
| phase-13: Wire TUI into CLI | Complete | 1 | 1 |
| phase-14: Enable Signal Handling | Complete | 1 | 1 |
| phase-15: Deterministic Markdown Gene... | Complete | 1 | 1 |
| phase-15.5: Phase-Level Build Verification | Complete | 1 | 1 |
| phase-16: Sanity Check Workflow | Complete | 6 | 6 |
| phase-17: Concise One-at-a-Time Quest... | Complete | 4 | 4 |
| phase-18: Regenerate MD After Skills | Complete | 2 | 2 |
| phase-19: Skills No Auto-Implement | Complete | 3 | 3 |
| phase-20: Simplify Skill Input | Complete | 3 | 3 |
| phase-21: Agent Templates & Project D... | Complete | 4 | 4 |
| phase-22: TUI Default Mode + Interact... | Complete | 5 | 5 |
| phase-23: Detailed File Logging | Complete | 7 | 7 |
| phase-24: /end Skill | Complete | 6 | 6 |
| phase-25: Labeled Stderr Output | Complete | 3 | 3 |
| phase-26: Skills to Commands Migration | Complete | 7 | 7 |
| phase-27: Per-Phase TRACE Logging | Complete | 6 | 6 |
| phase-28: FIX Cycle Retry & Resume | Complete | 1 | 1 |
| phase-29: Per-Phase Agent Spawning | Complete | 1 | 1 |
| phase-30: Fix Roadmap Subitem Updates | Pending | 1 | 0 |
| **Total** | | **77** | **76** |
