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

## Phase 15.5: Phase-Level Build Verification

Status: **Complete** (1/1)

- [x] Move build verification from per-task to per-phase execution
  - [x] Add state helper methods (all_phase_tasks_completed, find_phase_for_task, mark_phase_status, add_task_to_phase)
  - [x] Update next_runnable_task for phase gating
  - [x] Remove per-task verify_all() from execute_implem and execute_fix
  - [x] Add check_phase_completion and inject_build_fix_task methods
  - [x] Integrate into run(), run_with_channels(), and step() loops

---

## Phase 16: Sanity Check Workflow

Status: **Complete** (6/6)

- [x] Create src/state/validate.rs
  - [x] SanityError enum with thiserror
  - [x] JSON syntax validation
  - [x] Schema compliance validation
  - [x] Cross-reference validation (roadmap_item_id, linked_task_ids)
  - [x] Duplicate ID detection
  - [x] Orphaned reference detection
- [x] Add --sanity-check flag to CLI
  - [x] Add flag to Cli struct
  - [x] Implement execute_sanity_check()
  - [x] Display validation results with colors
- [x] Update assets/cm.md skill workflow
  - [x] Add Step 7: Run cm --sanity-check after generation
  - [x] Add iteration loop if validation fails
  - [x] Add Step 8: Ask about .gitignore (default no)
  - [x] Add Step 9: Auto-generate commit message
  - [x] Add Step 10: Ask user confirmation and commit
- [x] Add unit tests for validation
  - [x] Test valid JSON files pass
  - [x] Test invalid JSON syntax detected
  - [x] Test missing required fields detected
  - [x] Test invalid cross-references detected
  - [x] Test duplicate IDs detected
  - [x] Test orphaned references detected
- [x] Add sanity check to assets/feat.md
  - [x] Add Step 9: Validate Changes after file updates
  - [x] Run cm --sanity-check, fix errors, repeat until pass
  - [x] Renumber Step 9 (Completion) to Step 10
- [x] Add sanity check to assets/fix.md
  - [x] Add Step 8: Validate Changes after file updates
  - [x] Run cm --sanity-check, fix errors, repeat until pass
  - [x] Renumber Step 8 (Completion) to Step 9

---

## Phase 17: Concise One-at-a-Time Questions in Skills

Status: **Complete** (4/4)

- [x] Refactor assets/cm.md questions
  - [x] Split Step 1 into 3 single questions
  - [x] Split Step 2 into 4 single questions
  - [x] Keep Step 3 as single optional question
  - [x] Keep Step 4 as single question
  - [x] Keep Step 5 confirmation as single block
- [x] Refactor assets/feat.md questions
  - [x] Split Step 1 into 3 single questions
  - [x] Split Step 2 into 4 single questions
  - [x] Split Step 5 into 4 single questions
  - [x] Keep confirmation as single block
- [x] Refactor assets/fix.md questions
  - [x] Split Step 1 into 4 single questions
  - [x] Split Step 2 into 2 single questions
  - [x] Keep confirmation as single block
- [x] Rebuild and verify skills
  - [x] cargo build --release
  - [x] cm init --force
  - [x] Test /cm skill manually
  - [x] Test /feat skill manually
  - [x] Test /fix skill manually

---

## Phase 18: Regenerate MD After Skills

Status: **Complete** (2/2)

- [x] Update assets/feat.md with regeneration step
  - [x] Add step to run cm --regenerate after file updates
  - [x] Place before sanity check step
- [x] Update assets/fix.md with regeneration step
  - [x] Add step to run cm --regenerate after file updates
  - [x] Place before sanity check step

---

## Phase 19: Skills No Auto-Implement

Status: **Complete** (3/3)

- [x] Update assets/feat.md scope limitations
  - [x] Add CRITICAL: Scope Limitations section
  - [x] Update file update step to only edit JSON
  - [x] Remove cm run from next steps
- [x] Update assets/fix.md scope limitations
  - [x] Add CRITICAL: Scope Limitations section
  - [x] Update file update step to only edit JSON
  - [x] Remove cm run from next steps
- [x] Reinstall skills and verify
  - [x] cargo build
  - [x] cm init --force
  - [x] Verify feat skill has scope limitations
  - [x] Verify fix skill has scope limitations

---

## Phase 20: Simplify Skill Input

Status: **Complete** (3/3)

- [x] Simplify assets/feat.md input
  - [x] Replace Step 1 to only ask for description
  - [x] Remove user story references
  - [x] Update templates
- [x] Simplify assets/fix.md input
  - [x] Replace Step 1 to only ask for description
  - [x] Remove summary/observed/expected/repro prompts
- [x] Reinstall and verify simplified skills
  - [x] cargo build
  - [x] cm init --force
  - [x] Verify feat skill simplified input
  - [x] Verify fix skill simplified input

---

## Phase 21: Agent Templates & Project Documentation

Status: **Complete** (4/4)

- [x] Create default template assets and update cm init
  - [x] Create assets/templates/ with 5 template files
  - [x] Update src/command.rs with new constants
  - [x] Update src/cli/init.rs to write templates to .cm/
- [x] Load agent templates from disk at runtime
  - [x] Update src/agent/prompt.rs to read .cm/agents/*.md
  - [x] Auto-install missing templates from embedded defaults
- [x] Update /cm skill to customize templates
  - [x] Add step to customize ACTIONS.md with commands
  - [x] Add step to customize STRUCTURE.md
- [x] Test and verify template system
  - [x] Test cm init creates templates
  - [x] Test runtime auto-install of missing templates
  - [x] Test user customizations are preserved

---

## Phase 22: TUI Default Mode + Interactive Task Selection

Status: **Complete** (5/5)

- [x] Update CLI: TUI default, add --daemon flag
  - [x] Remove --tui flag
  - [x] Add --daemon flag
  - [x] Invert execution logic
- [x] Add task selection prompt to manager
  - [x] Show pending tasks before running
  - [x] Prompt: [s]ingle / [a]ll / [q]uit
  - [x] Never auto-implement without confirmation
- [x] Implement TUI task selection widget
  - [x] Task list with pending highlighted
  - [x] Keyboard controls: s/a/q/arrows
  - [x] Prompt bar at bottom
- [x] Implement daemon mode stdin prompt
  - [x] Print pending tasks to stdout
  - [x] Read selection from stdin
  - [x] Prompt again after task completes
- [x] Update tests for new default mode
  - [x] Update CLI tests for --daemon
  - [x] Update integration tests
  - [x] Verify all tests pass

---

## Phase 23: Detailed File Logging

Status: **Complete** (7/7)

- [x] Create FileLogger module
  - [x] LogLevel enum with Display and PartialOrd
  - [x] FileLogError enum with thiserror
  - [x] FileLogger struct with Mutex<File>
  - [x] Core methods: new(), with_level(), log()
  - [x] Convenience methods: debug(), info(), warn(), error()
- [x] Add prune functionality
  - [x] PruneStats struct
  - [x] parse_log_timestamp() helper
  - [x] prune() method (24h default)
  - [x] prune_older_than() method
- [x] Export FileLogger from log module
  - [x] Add mod file_logger declaration
  - [x] Add pub use re-exports
- [x] Add --prune CLI flag
  - [x] Add prune flag to Cli struct
  - [x] Implement execute_prune() function
  - [x] Wire up in CLI dispatch
- [x] Add FileLogger to Manager
  - [x] Add file_log_path to ManagerConfig
  - [x] Add verbose flag to ManagerConfig
  - [x] Add file_logger field to Manager
  - [x] Initialize FileLogger in Manager::new()
- [x] Add logging calls throughout Manager
  - [x] Log state load/save operations
  - [x] Log task selection and completion
  - [x] Log agent spawn and completion with duration
  - [x] Log build verification results
  - [x] Log shutdown and signal handling
- [x] Add unit tests for FileLogger
  - [x] Test file creation and log levels
  - [x] Test log format
  - [x] Test timestamp parsing
  - [x] Test pruning functionality
  - [x] Test concurrent logging

---

## Phase 24: /end Skill

Status: **Complete** (6/6)

- [x] Create assets/end.md skill
  - [x] YAML frontmatter (name, description, user-invocable)
  - [x] Skill body with save pipeline instructions
  - [x] Scope limitations (no implementation)
  - [x] Error handling sections
- [x] Register END_SKILL in src/skill.rs
- [x] Register end skill in src/cli/init.rs
  - [x] Add END_SKILL import
  - [x] Add SkillFile entry to SKILLS array
  - [x] Add /end to available skills output
- [x] Modify assets/feat.md to delegate saves to /end
  - [x] Remove Steps 8-11 (file updates, regenerate, validate, completion)
  - [x] Add Step 8: Handoff to /end
- [x] Modify assets/fix.md to delegate saves to /end
  - [x] Remove Steps 7-10 (file updates, regenerate, validate, completion)
  - [x] Add Step 7: Handoff to /end
- [x] Build, reinstall skills, and verify
  - [x] cargo build + clippy + test pass
  - [x] cm init --force installs /end skill
  - [x] Verify feat/fix skills have /end handoff

---

## Phase 25: Labeled Stderr Output

Status: **Complete** (3/3)

- [x] Thread agent_label through AgentSpawner
  - [x] Add agent_label param to spawn()
  - [x] Add agent_label param to spawn_with_continue()
  - [x] Use label in run_agent_thread() eprint lines
- [x] Pass agent labels at spawn sites and add [CM] eprints
  - [x] Pass IMPLEM/REVIEW/FIX at 6 spawn call sites
  - [x] Replace inline [AGENT] eprints with specific labels
  - [x] Add emit_cm() helper and [CM] orchestration eprints
- [x] Build and verify labeled output
  - [x] cargo build + clippy + test pass
  - [x] No remaining hardcoded AGENT strings in eprints

---

## Phase 26: Skills to Commands Migration

Status: **Complete** (7/7)

- [x] Remove YAML front matter and update skill->command text in assets
  - [x] Strip YAML front matter from assets/cm.md
  - [x] Strip YAML front matter from assets/feat.md
  - [x] Strip YAML front matter from assets/fix.md
  - [x] Strip YAML front matter from assets/end.md
  - [x] Replace 'This skill' with 'This command' in all assets
- [x] Rename src/skill.rs to src/command.rs
  - [x] Delete src/skill.rs
  - [x] Create src/command.rs with *_COMMAND constants
  - [x] Update src/lib.rs module declaration
- [x] Update CLI help text
- [x] Rewrite src/cli/init.rs for commands
  - [x] Rename SkillFile to CommandFile, SKILLS to COMMANDS
  - [x] Install to .claude/commands/{name}.md (flat files)
  - [x] Add cleanup_legacy_skills() function
  - [x] Rewrite all tests for new path structure
- [x] Update README.md
  - [x] Rename Skills section to Commands
  - [x] Update directory tree
  - [x] Add /end to commands table
- [x] Delete legacy .claude/skills/ directory
  - [x] Remove .claude/skills/ tree
  - [x] Run cm init to create .claude/commands/
- [x] Build and verify commands migration
  - [x] cargo build + clippy + test pass
  - [x] Verify src/skill.rs deleted, src/command.rs exists
  - [x] Verify .claude/commands/ exists, .claude/skills/ gone

---

## Phase 27: Per-Phase TRACE Logging

Status: **Complete** (6/6)

- [x] Add TRACE level to LogLevel enum
  - [x] Add Trace variant before Debug
  - [x] Update Display impl
  - [x] Add trace() convenience method
  - [x] Update level ordering tests
- [x] Create PhaseLogger struct
  - [x] Create src/log/phase_logger.rs
  - [x] Implement extract_phase_id() helper
  - [x] Implement log_prompt() method
  - [x] Implement log_response() method
  - [x] Export from log module
- [x] Integrate PhaseLogger into Manager
  - [x] Add phase_logger field to Manager
  - [x] Log prompts in execute_implem/review/fix
  - [x] Log responses in execute_implem/review/fix
  - [x] Log in run_review_cycle
- [x] Update --prune for per-phase logs
  - [x] Add prune_all() function
  - [x] Update execute_prune() in CLI
- [x] Add unit tests for PhaseLogger
  - [x] Test extract_phase_id
  - [x] Test directory creation
  - [x] Test log file creation and format
  - [x] Test multiple phases create separate files
- [x] Build and verify per-phase logging
  - [x] cargo build + clippy + test pass
  - [x] Verify PhaseLogger integration
  - [x] Verify TRACE level added

---

## Phase 28: FIX Cycle Retry & Resume

Status: **Complete** (1/1)

- [x] Interactive retry prompt after max FIX cycles + resume at REVIEW on --continue
  - [x] Add implem_completed_at, baseline_commit, review_cycles_completed to Task
  - [x] Add RetryPrompt/RetryResponse to TUI events/commands
  - [x] Implement prompt_retry_cycles() for TUI and daemon modes
  - [x] Modify run_review_cycle() to prompt on max cycles
  - [x] Save IMPLEM completion state before review cycle
  - [x] Resume at REVIEW on --continue when IMPLEM completed

---

## Phase 30: Fix Roadmap Subitem Updates

Status: **Complete** (1/1)

- [x] Fix roadmap subitem ID lookup
  - [x] Add optional id field to RoadmapSubItem
  - [x] Add ensure_subitem_ids() method
  - [x] Update load_roadmap() for backward compatibility
  - [x] Update sync_roadmap_item() to use ID-based lookup
  - [x] Update validation to recognize subitem IDs

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
| Phase 15.5: Phase-Level Build Verification | Complete | 6/6 |
| Phase 16: Sanity Check Workflow | Complete | 32/32 |
| Phase 17: Concise One-at-a-Time Questions in Skills | Complete | 21/21 |
| Phase 18: Regenerate MD After Skills | Complete | 6/6 |
| Phase 19: Skills No Auto-Implement | Complete | 13/13 |
| Phase 20: Simplify Skill Input | Complete | 12/12 |
| Phase 21: Agent Templates & Project Documentation | Complete | 14/14 |
| Phase 22: TUI Default Mode + Interactive Task Selection | Complete | 20/20 |
| Phase 23: Detailed File Logging | Complete | 35/35 |
| Phase 24: /end Skill | Complete | 20/20 |
| Phase 25: Labeled Stderr Output | Complete | 11/11 |
| Phase 26: Skills to Commands Migration | Complete | 27/27 |
| Phase 27: Per-Phase TRACE Logging | Complete | 28/28 |
| Phase 28: FIX Cycle Retry & Resume | Complete | 7/7 |
| Phase 30: Fix Roadmap Subitem Updates | Complete | 6/6 |
| **Total** | | **349/349** |
