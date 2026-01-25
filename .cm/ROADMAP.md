# Claude Code Manager - Roadmap

This document tracks implementation progress. Check off items as they are completed.

## Phase 0: Project Setup

Status: **Complete** (4/4)

- [x] Create Cargo.toml with workspace configuration
  - [x] Package metadata (name, version, edition, description)
  - [x] Dependencies: clap, serde, serde_json, thiserror, chrono, uuid, log, env_logger
  - [x] TUI dependencies: ratatui, crossterm
  - [x] Dev dependencies: tempfile, assert_cmd
- [x] Create src/lib.rs with module declarations
- [x] Create src/main.rs with CLI skeleton
- [x] Verify initial build
  - [x] cargo build passes
  - [x] cargo clippy passes

---

## Phase 0.5: cm init + Skills

Status: **Complete** (5/5)

- [x] Create assets/ directory with all skills
- [x] Embed all skills in binary
- [x] Add init subcommand
- [x] Create /feat skill content
- [x] Create /fix skill content

---

## Phase 1: State Types

Status: **Complete** (7/7)

- [x] Create src/state/mod.rs
- [x] Create src/state/tasks.rs with core types
- [x] Create Task type
- [x] Create TaskAttempt and related types
- [x] Create AgentInvocation type
- [x] Implement Serialize/Deserialize
- [x] Add helper methods to TasksState

---

## Phase 2: Agent Module

Status: **Complete** (5/5)

- [x] Create src/agent/mod.rs
- [x] Implement AgentSpawner
- [x] Create AgentHandle type
- [x] Create src/agent/prompt.rs
- [x] Create src/agent/response.rs

---

## Phase 3: Build Module

Status: **Complete** (4/4)

- [x] Create src/build/mod.rs
- [x] Create src/build/cargo.rs
- [x] Create src/build/git.rs
- [x] Implement BuildVerifier

---

## Phase 4: Log Module

Status: **Complete** (4/4)

- [x] Create src/log/mod.rs
- [x] Implement LogManager
- [x] Implement log formatting
- [x] Manager calls LogManager

---

## Phase 5: Manager Core

Status: **Complete** (8/8)

- [x] Create src/manager/mod.rs
- [x] Implement Manager struct
- [x] Implement main orchestration loop
- [x] Implement IMPLEM flow
- [x] Implement REVIEW flow
- [x] Implement FIX flow
- [x] Implement cycle limiting
- [x] Create src/manager/state.rs

---

## Phase 6: Recovery

Status: **Complete** (5/5)

- [x] Create src/manager/recovery.rs
- [x] Implement checkpointing
- [x] Implement restore
- [x] Implement crash recovery
- [x] Implement graceful shutdown

---

## Phase 7: CLI Commands

Status: **Complete** (7/7)

- [x] Create src/cli/mod.rs
- [x] Implement main execution
- [x] Implement --continue flag
- [x] Implement --step flag
- [x] Implement --status flag
- [x] Implement --validate flag
- [x] Wire up main.rs

---

## Phase 8: Claude Code Skill

Status: **Complete** (4/4)

- [x] Create .claude/skills/cm.md
- [x] Define interactive wizard flow
- [x] Implement artifact generation
- [x] Create generation templates

---

## Phase 9: Integration Testing

Status: **Complete** (4/4)

- [x] Create tests/integration.rs
- [x] Test basic flow
- [x] Test recovery
- [x] Test deferred tasks

---

## Phase 10: Terminal UI

Status: **Complete** (8/8)

- [x] Add ratatui dependency
- [x] Create src/tui/mod.rs
- [x] Create src/tui/layout.rs
- [x] Create src/tui/widgets.rs
- [x] Implement task list view
- [x] Implement agent stream view
- [x] Implement keyboard controls
- [x] Wire TUI into Manager

---

## Phase 11: Polish

Status: **Complete** (3/3)

- [x] Add --dry-run mode
- [x] Add --verbose mode
- [x] Final verification

---

## Phase 12: Configuration File Support

Status: **Complete** (4/4)

- [x] Define configuration file format
- [x] Implement config file loading
- [x] Wire --config flag in CLI
- [x] Create CONFIG.md documentation

---

## Phase 13: Wire TUI into CLI

Status: **Complete** (3/3)

- [x] Add --tui flag to CLI
- [x] Connect TUI to Manager
- [x] Run Manager in background thread

---

## Phase 14: Enable Signal Handling

Status: **Complete** (2/2)

- [x] Enable ctrlc by default
- [x] Wire signal handler in CLI

---

## Phase 15: Deterministic Markdown Generation

Status: **Complete** (8/8)

- [x] Create LogRecord struct
- [x] Extend TasksState with log_records
- [x] Create roadmap.json schema
- [x] Add roadmap link to Task
- [x] Create markdown generators
- [x] Refactor LogManager
- [x] Integrate regeneration into Manager
- [x] Add --regenerate CLI flag

---

## Phase 16: Sanity Check Workflow

Status: **Not Started** (0/6)

- [ ] Create src/state/validate.rs
  - [ ] SanityError enum with thiserror
  - [ ] JSON syntax validation
  - [ ] Schema compliance validation
  - [ ] Cross-reference validation (roadmap_item_id, linked_task_ids)
  - [ ] Duplicate ID detection
  - [ ] Orphaned reference detection
- [ ] Add --sanity-check flag to CLI
  - [ ] Add flag to Cli struct
  - [ ] Implement execute_sanity_check()
  - [ ] Display validation results with colors
- [ ] Update assets/cm.md skill workflow
  - [ ] Add Step 7: Run cm --sanity-check after generation
  - [ ] Add iteration loop if validation fails
  - [ ] Add Step 8: Ask about .gitignore (default no)
  - [ ] Add Step 9: Auto-generate commit message
  - [ ] Add Step 10: Ask user confirmation and commit
- [ ] Add unit tests for validation
  - [ ] Test valid JSON files pass
  - [ ] Test invalid JSON syntax detected
  - [ ] Test missing required fields detected
  - [ ] Test invalid cross-references detected
  - [ ] Test duplicate IDs detected
  - [ ] Test orphaned references detected
- [ ] Add sanity check to assets/feat.md
  - [ ] Add Step 9: Validate Changes after file updates
  - [ ] Run cm --sanity-check, fix errors, repeat until pass
  - [ ] Renumber Step 9 (Completion) to Step 10
- [ ] Add sanity check to assets/fix.md
  - [ ] Add Step 8: Validate Changes after file updates
  - [ ] Run cm --sanity-check, fix errors, repeat until pass
  - [ ] Renumber Step 8 (Completion) to Step 9

---

## Phase 17: Concise One-at-a-Time Questions in Skills

Status: **Not Started** (0/4)

- [ ] Refactor assets/cm.md questions
  - [ ] Split Step 1 into 3 single questions
  - [ ] Split Step 2 into 4 single questions
  - [ ] Keep Step 3 as single optional question
  - [ ] Keep Step 4 as single question
  - [ ] Keep Step 5 confirmation as single block
- [ ] Refactor assets/feat.md questions
  - [ ] Split Step 1 into 3 single questions
  - [ ] Split Step 2 into 4 single questions
  - [ ] Split Step 5 into 4 single questions
  - [ ] Keep confirmation as single block
- [ ] Refactor assets/fix.md questions
  - [ ] Split Step 1 into 4 single questions
  - [ ] Split Step 2 into 2 single questions
  - [ ] Keep confirmation as single block
- [ ] Rebuild and verify skills
  - [ ] cargo build --release
  - [ ] cm init --force
  - [ ] Test /cm skill manually
  - [ ] Test /feat skill manually
  - [ ] Test /fix skill manually

---

## Phase 18: Regenerate MD After Skills

Status: **Not Started** (0/2)

- [ ] Update assets/feat.md with regeneration step
  - [ ] Add step to run cm --regenerate after file updates
  - [ ] Place before sanity check step
- [ ] Update assets/fix.md with regeneration step
  - [ ] Add step to run cm --regenerate after file updates
  - [ ] Place before sanity check step

---

## Phase 19: Skills No Auto-Implement

Status: **Not Started** (0/3)

- [ ] Update assets/feat.md scope limitations
  - [ ] Add CRITICAL: Scope Limitations section
  - [ ] Update file update step to only edit JSON
  - [ ] Remove cm run from next steps
- [ ] Update assets/fix.md scope limitations
  - [ ] Add CRITICAL: Scope Limitations section
  - [ ] Update file update step to only edit JSON
  - [ ] Remove cm run from next steps
- [ ] Reinstall skills and verify
  - [ ] cargo build
  - [ ] cm init --force
  - [ ] Verify feat skill has scope limitations
  - [ ] Verify fix skill has scope limitations

---

## Phase 20: Simplify Skill Input

Status: **Not Started** (0/3)

- [ ] Simplify assets/feat.md input
  - [ ] Replace Step 1 to only ask for description
  - [ ] Remove user story references
  - [ ] Update templates
- [ ] Simplify assets/fix.md input
  - [ ] Replace Step 1 to only ask for description
  - [ ] Remove summary/observed/expected/repro prompts
- [ ] Reinstall and verify simplified skills
  - [ ] cargo build
  - [ ] cm init --force
  - [ ] Verify feat skill simplified input
  - [ ] Verify fix skill simplified input

---

## Phase 21: Agent Templates & Project Documentation

Status: **Not Started** (0/3)

- [ ] Add agent templates generation to /cm skill
  - [ ] Generate .cm/agents/MANAGER.md
  - [ ] Generate .cm/agents/IMPLEMENTER.md
  - [ ] Generate .cm/agents/REVIEWER.md
- [ ] Add STRUCTURE.md and ACTIONS.md generation
  - [ ] Add questions for structure info
  - [ ] Add questions for actions info
  - [ ] Generate .cm/STRUCTURE.md
  - [ ] Generate .cm/ACTIONS.md
- [ ] Reinstall and verify new /cm outputs
  - [ ] cargo build
  - [ ] cm init --force
  - [ ] Verify agent templates generated
  - [ ] Verify STRUCTURE.md and ACTIONS.md generated

---

## Phase 22: TUI Default Mode + Interactive Task Selection

Status: **Not Started** (0/5)

- [ ] Update CLI: TUI default, add --daemon flag
  - [ ] Remove --tui flag
  - [ ] Add --daemon flag
  - [ ] Invert execution logic
- [ ] Add task selection prompt to manager
  - [ ] Show pending tasks before running
  - [ ] Prompt: [s]ingle / [a]ll / [q]uit
  - [ ] Never auto-implement without confirmation
- [ ] Implement TUI task selection widget
  - [ ] Task list with pending highlighted
  - [ ] Keyboard controls: s/a/q/arrows
  - [ ] Prompt bar at bottom
- [ ] Implement daemon mode stdin prompt
  - [ ] Print pending tasks to stdout
  - [ ] Read selection from stdin
  - [ ] Prompt again after task completes
- [ ] Update tests for new default mode
  - [ ] Update CLI tests for --daemon
  - [ ] Update integration tests
  - [ ] Verify all tests pass

---

## Phase 23: Detailed File Logging

Status: **Not Started** (0/7)

- [ ] Create FileLogger module
  - [ ] LogLevel enum with Display and PartialOrd
  - [ ] FileLogError enum with thiserror
  - [ ] FileLogger struct with Mutex<File>
  - [ ] Core methods: new(), with_level(), log()
  - [ ] Convenience methods: debug(), info(), warn(), error()
- [ ] Add prune functionality
  - [ ] PruneStats struct
  - [ ] parse_log_timestamp() helper
  - [ ] prune() method (24h default)
  - [ ] prune_older_than() method
- [ ] Export FileLogger from log module
  - [ ] Add mod file_logger declaration
  - [ ] Add pub use re-exports
- [ ] Add --prune CLI flag
  - [ ] Add prune flag to Cli struct
  - [ ] Implement execute_prune() function
  - [ ] Wire up in CLI dispatch
- [ ] Add FileLogger to Manager
  - [ ] Add file_log_path to ManagerConfig
  - [ ] Add verbose flag to ManagerConfig
  - [ ] Add file_logger field to Manager
  - [ ] Initialize FileLogger in Manager::new()
- [ ] Add logging calls throughout Manager
  - [ ] Log state load/save operations
  - [ ] Log task selection and completion
  - [ ] Log agent spawn and completion with duration
  - [ ] Log build verification results
  - [ ] Log shutdown and signal handling
- [ ] Add unit tests for FileLogger
  - [ ] Test file creation and log levels
  - [ ] Test log format
  - [ ] Test timestamp parsing
  - [ ] Test pruning functionality
  - [ ] Test concurrent logging

---

## Phase 24: /end Skill

Status: **Not Started** (0/6)

- [ ] Create assets/end.md skill
  - [ ] YAML frontmatter (name, description, user-invocable)
  - [ ] Skill body with save pipeline instructions
  - [ ] Scope limitations (no implementation)
  - [ ] Error handling sections
- [ ] Register END_SKILL in src/skill.rs
- [ ] Register end skill in src/cli/init.rs
  - [ ] Add END_SKILL import
  - [ ] Add SkillFile entry to SKILLS array
  - [ ] Add /end to available skills output
- [ ] Modify assets/feat.md to delegate saves to /end
  - [ ] Remove Steps 8-11 (file updates, regenerate, validate, completion)
  - [ ] Add Step 8: Handoff to /end
- [ ] Modify assets/fix.md to delegate saves to /end
  - [ ] Remove Steps 7-10 (file updates, regenerate, validate, completion)
  - [ ] Add Step 7: Handoff to /end
- [ ] Build, reinstall skills, and verify
  - [ ] cargo build + clippy + test pass
  - [ ] cm init --force installs /end skill
  - [ ] Verify feat/fix skills have /end handoff

---

## Summary

| Phase | Status | Progress |
|-------|--------|----------|
| Phase 0: Project Setup | Complete | 10/10 |
| Phase 0.5: cm init + Skills | Complete | 5/5 |
| Phase 1: State Types | Complete | 7/7 |
| Phase 2: Agent Module | Complete | 5/5 |
| Phase 3: Build Module | Complete | 4/4 |
| Phase 4: Log Module | Complete | 4/4 |
| Phase 5: Manager Core | Complete | 8/8 |
| Phase 6: Recovery | Complete | 5/5 |
| Phase 7: CLI Commands | Complete | 7/7 |
| Phase 8: Claude Code Skill | Complete | 4/4 |
| Phase 9: Integration Testing | Complete | 4/4 |
| Phase 10: Terminal UI | Complete | 8/8 |
| Phase 11: Polish | Complete | 3/3 |
| Phase 12: Configuration File Support | Complete | 4/4 |
| Phase 13: Wire TUI into CLI | Complete | 3/3 |
| Phase 14: Enable Signal Handling | Complete | 2/2 |
| Phase 15: Deterministic Markdown Generation | Complete | 8/8 |
| Phase 16: Sanity Check Workflow | Not Started | 0/32 |
| Phase 17: Concise One-at-a-Time Questions in Skills | Not Started | 0/21 |
| Phase 18: Regenerate MD After Skills | Not Started | 0/6 |
| Phase 19: Skills No Auto-Implement | Not Started | 0/13 |
| Phase 20: Simplify Skill Input | Not Started | 0/12 |
| Phase 21: Agent Templates & Project Documentation | Not Started | 0/14 |
| Phase 22: TUI Default Mode + Interactive Task Selection | Not Started | 0/20 |
| Phase 23: Detailed File Logging | Not Started | 0/35 |
| Phase 24: /end Skill | Not Started | 0/20 |
| **Total** | | **91/264** |
