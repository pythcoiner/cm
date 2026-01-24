# Claude Code Manager - Implementation Roadmap

This is the detailed checklist of all implementation tasks. Check items off as they are completed.

---

## Phase 0: Project Setup

- [x] **0.1** Create Cargo.toml with workspace configuration
  - [x] Package metadata (name, version, edition, description)
  - [x] Dependencies: clap, serde, serde_json, thiserror, chrono, uuid, log, env_logger
  - [x] TUI dependencies: ratatui, crossterm
  - [x] Dev dependencies: tempfile, assert_cmd
  - [x] NO tokio - use std::thread, std::sync::mpsc, std::process

- [x] **0.2** Create src/lib.rs with module declarations
  - [x] Declare modules: manager, state, agent, build, log, tui, cli
  - [x] Re-export key types

- [x] **0.3** Create src/main.rs with CLI skeleton
  - [x] Clap derive for CLI struct
  - [x] Flags: --continue, --step, --status, --validate
  - [x] Options: --verbose, --config, --state

- [x] **0.4** Verify initial build
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes

---

## Phase 1: State Types

- [x] **1.1** Create src/state/mod.rs
  - [x] Module declarations
  - [x] StateError enum with thiserror
  - [x] load_state() function
  - [x] save_state() function

- [x] **1.2** Create src/state/tasks.rs with core types
  - [x] TasksState struct (version, project, phases, current_phase, current_task, agent_history)
  - [x] Project struct (name, description, created_at)
  - [x] GlobalContext struct (plan_summary)
  - [x] Phase struct (id, name, status, tasks)
  - [x] PhaseStatus enum (Pending, InProgress, Completed)

- [x] **1.3** Create Task type
  - [x] Task struct (id, name, task_type, status, depends_on, context, instructions, attempts)
  - [x] TaskType enum (Implement, Review, Fix, Test)
  - [x] TaskStatus enum (Pending, InProgress, Completed, Deferred)
  - [x] TaskContext struct (files_to_read, code_style_excerpt, prior_review_issues)

- [x] **1.4** Create TaskAttempt and related types
  - [x] TaskAttempt struct (attempt_number, agent_id, started_at, completed_at, status, response)
  - [x] AttemptStatus enum (Success, Failed, Timeout)
  - [x] AgentResponse struct (files_created, files_modified, commands_run, raw_response)
  - [x] ReviewResult struct (verdict, issues)
  - [x] ReviewIssue struct (id, severity, location, problem, suggested_fix, resolved)
  - [x] Severity enum (Critical, High, Medium, Low)
  - [x] Verdict enum (Approved, NeedsFixes)

- [x] **1.5** Create AgentInvocation type
  - [x] AgentInvocation struct (id, task_id, agent_type, started_at, completed_at, exit_status)
  - [x] AgentType enum (Main, Implem, Review, Fix)

- [x] **1.6** Implement Serialize/Deserialize for all types
  - [x] Derive serde traits
  - [x] Use rename_all = "snake_case" for enums
  - [x] DateTime<Utc> serialization

- [x] **1.7** Add helper methods to TasksState
  - [x] current_phase() -> Option<&Phase>
  - [x] current_task() -> Option<&Task>
  - [x] next_runnable_task() -> Option<&Task>
  - [x] is_task_blocked(task_id) -> bool
  - [x] mark_task_status(task_id, status)

- [x] **1.8** Verify state module
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes
  - [x] Unit tests for serialization round-trip

---

## Phase 2: Agent Module

- [x] **2.1** Create src/agent/mod.rs
  - [x] Module declarations
  - [x] AgentError enum with thiserror
  - [x] AgentSpawner struct

- [x] **2.2** Implement AgentSpawner
  - [x] new(model, timeout) constructor
  - [x] spawn(prompt, task_id) -> Result<AgentHandle>
  - [x] Use std::process::Command for `claude -p`
  - [x] Capture stdout/stderr
  - [x] Return AgentHandle with process handle

- [x] **2.3** Create AgentHandle type
  - [x] AgentHandle struct (thread handle, stop_flag, task_id, started_at)
  - [x] wait() -> Result<AgentOutput> (blocks until thread completes)
  - [x] interrupt() -> kill child process via stop_flag
  - [x] Use std::thread, NOT async

- [x] **2.4** Create src/agent/prompt.rs
  - [x] PromptBuilder struct
  - [x] build_implem_prompt(task) -> String
  - [x] build_review_prompt(task, code_to_review) -> String
  - [x] build_fix_prompt(task, issues) -> String
  - [x] Ensure NO global context leaks into prompts

- [x] **2.5** Create src/agent/response.rs
  - [x] ResponseParser struct
  - [x] parse(raw_json) -> Result<AgentResponse>
  - [x] Handle malformed JSON gracefully
  - [x] Extract files_created, files_modified, commands_run

- [x] **2.6** Verify agent module
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes
  - [x] Unit tests for prompt building

---

## Phase 3: Build Module

- [x] **3.1** Create src/build/mod.rs
  - [x] Module declarations
  - [x] BuildError enum with thiserror
  - [x] BuildVerifier struct

- [x] **3.2** Create src/build/cargo.rs
  - [x] CargoRunner struct
  - [x] build() -> Result<BuildOutput>
  - [x] clippy() -> Result<ClippyOutput>
  - [x] test() -> Result<TestOutput>
  - [x] Parse cargo output for errors/warnings

- [x] **3.3** Create src/build/git.rs
  - [x] GitRunner struct
  - [x] status() -> Result<GitStatus>
  - [x] commit(message) -> Result<CommitId>
  - [x] add(files) -> Result<()>

- [x] **3.4** Implement BuildVerifier
  - [x] new() constructor
  - [x] verify_build() -> Result<()>
  - [x] verify_clippy() -> Result<()>
  - [x] Run build then clippy, collect all errors

- [x] **3.5** Verify build module
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes

---

## Phase 4: Log Module

**IMPORTANT:** cm (manager) owns LOG.md - agents never touch it.

- [x] **4.1** Create src/log/mod.rs
  - [x] LogError enum with thiserror
  - [x] LogManager struct

- [x] **4.2** Implement LogManager
  - [x] new(path) constructor
  - [x] append_entry(entry) -> Result<()>
  - [x] LogEntry struct (timestamp, phase, task, agent_id, action, details)

- [x] **4.3** Implement log formatting
  - [x] format_phase_start(phase) -> String
  - [x] format_agent_spawn(agent_type, task_id, prompt) -> String
  - [x] format_agent_response(response) -> String
  - [x] format_build_result(output) -> String
  - [x] format_review_result(verdict, issues) -> String

- [x] **4.4** Manager calls LogManager
  - [x] Log before spawning agent (prompt)
  - [x] Log after agent completes (response)
  - [x] Log build/test results
  - [x] Log task completion/deferral

- [x] **4.5** Verify log module
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes

---

## Phase 5: Manager Core

- [x] **5.1** Create src/manager/mod.rs
  - [x] Module declarations
  - [x] ManagerError enum with thiserror
  - [x] Manager struct

- [x] **5.2** Implement Manager struct
  - [x] Fields: state, config, agent_spawner, build_verifier, log_manager
  - [x] new(config) -> Result<Self>
  - [x] load_or_create_state() -> Result<TasksState>

- [x] **5.3** Implement main orchestration loop
  - [x] run() -> Result<()>
  - [x] Loop: select_next_task() -> execute_task() -> update_state()
  - [x] Handle all task types (Implement, Review, Fix)
  - [x] Check dependencies before executing

- [x] **5.4** Implement IMPLEM flow
  - [x] Build prompt with task context only
  - [x] Spawn agent
  - [x] Wait for response
  - [x] Parse response
  - [x] Run build verification
  - [x] If build fails, create FIX task
  - [x] If build passes, create REVIEW task

- [x] **5.5** Implement REVIEW flow
  - [x] Build review prompt
  - [x] Spawn agent
  - [x] Parse review result
  - [x] If APPROVED, mark task complete
  - [x] If NEEDS_FIXES, create FIX task

- [x] **5.6** Implement FIX flow
  - [x] Build fix prompt with issues
  - [x] Spawn agent
  - [x] Parse response
  - [x] Run build verification
  - [x] Create REVIEW task

- [x] **5.7** Implement cycle limiting
  - [x] Track attempt count per task
  - [x] After 5 cycles, mark task as DEFERRED
  - [x] Continue with next non-blocked task

- [x] **5.8** Create src/manager/state.rs
  - [x] ManagerState enum (Idle, Executing, WaitingForAgent, Verifying)
  - [x] State transition methods

- [x] **5.9** Verify manager core
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes

---

## Phase 6: Recovery

- [x] **6.1** Create src/manager/recovery.rs
  - [x] RecoveryManager struct
  - [x] RecoveryError enum

- [x] **6.2** Implement checkpointing
  - [x] checkpoint() -> Result<CheckpointId>
  - [x] Save state before mutations
  - [x] Store in .cm/checkpoints/

- [x] **6.3** Implement restore
  - [x] restore(checkpoint_id) -> Result<()>
  - [x] Load state from checkpoint

- [x] **6.4** Implement crash recovery
  - [x] recover_from_crash() -> Result<RecoveryAction>
  - [x] Detect incomplete task
  - [x] Determine recovery action (retry, skip, etc.)

- [x] **6.5** Implement graceful shutdown
  - [x] Handle SIGINT/SIGTERM
  - [x] Wait for current agent (with timeout)
  - [x] Save state with interrupted marker
  - [x] Append shutdown to LOG.md

- [x] **6.6** Verify recovery module
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes

---

## Phase 7: CLI Commands

- [x] **7.1** Create src/cli/mod.rs
  - [x] Cli struct with clap derive
  - [x] Flags: --continue, --step, --status, --validate, --verbose
  - [x] Options: --config, --state

- [x] **7.2** Implement main execution (default)
  - [x] Load state from .cm/tasks.json
  - [x] Create Manager
  - [x] Call manager.run()

- [x] **7.3** Implement --continue flag
  - [x] Load state
  - [x] Call recovery.recover_from_crash()
  - [x] Resume execution

- [x] **7.4** Implement --step flag
  - [x] Execute one task only
  - [x] Save state and exit

- [x] **7.5** Implement --status flag
  - [x] Load state
  - [x] Display progress summary
  - [x] Show current phase/task
  - [x] Show deferred tasks

- [x] **7.6** Implement --validate flag
  - [x] Load tasks.json
  - [x] Validate against schema
  - [x] Report errors

- [x] **7.7** Wire up main.rs
  - [x] Parse CLI args
  - [x] Dispatch based on flags
  - [x] Set up logging (--verbose)

- [x] **7.8** Verify CLI
  - [x] `cargo build` passes
  - [x] `cargo clippy` passes
  - [x] `cm --help` works

---

## Phase 8: Claude Code Skill

- [x] **8.1** Create .claude/skills/cm.md
  - [x] Skill name and description
  - [x] Usage: /cm

- [x] **8.2** Define interactive wizard flow
  - [x] Ask: What do you want to build?
  - [x] Ask: What is the scope/goal?
  - [x] Ask: Any reference implementation to analyze?
  - [x] Ask: What phases do you see?
  - [x] Confirm understanding before generating

- [x] **8.3** Implement artifact generation
  - [x] Generate .cm/PLAN.md from conversation
  - [x] Generate .cm/ROADMAP.md with detailed tasks
  - [x] Generate .cm/tasks.json with all tasks and dependencies
  - [x] Initialize .cm/LOG.md

- [x] **8.4** Create generation templates
  - [x] PLAN.md template structure
  - [x] ROADMAP.md checklist format
  - [x] tasks.json schema with depends_on
  - [x] LOG.md header template

---

## Phase 9: Integration Testing

- [x] **9.1** Create tests/integration.rs
  - [x] Test setup helpers

- [x] **9.2** Test basic flow
  - [x] Create minimal tasks.json
  - [x] Run cm with mock agent
  - [x] Verify state updates

- [x] **9.3** Test recovery
  - [x] Simulate crash during execution
  - [x] Run cm continue
  - [x] Verify resumed correctly

- [x] **9.4** Test deferred tasks
  - [x] Create task that fails 5 times
  - [x] Verify marked as deferred
  - [x] Verify next task runs

---

## Phase 10: Terminal UI (ratatui)

- [ ] **10.1** Add ratatui dependency
  - [ ] ratatui = "0.26"
  - [ ] crossterm = "0.27" (backend)

- [ ] **10.2** Create src/tui/mod.rs
  - [ ] App struct (state for TUI)
  - [ ] run_tui(manager) -> Result<()>
  - [ ] Terminal setup/teardown

- [ ] **10.3** Create src/tui/layout.rs
  - [ ] Split view: left (tasks) + right (stream)
  - [ ] Bottom bar with controls
  - [ ] Responsive to terminal size

- [ ] **10.4** Create src/tui/widgets.rs
  - [ ] TaskListWidget - shows phases and tasks with status icons
  - [ ] StreamWidget - scrolling log of agent prompts/responses
  - [ ] ControlsWidget - shows keybindings

- [ ] **10.5** Implement task list view
  - [ ] Show all phases with tasks
  - [ ] Icons: ✓ completed, ⠋ in_progress, ○ pending, ⊘ deferred
  - [ ] Highlight current task
  - [ ] Scroll if too many tasks

- [ ] **10.6** Implement agent stream view
  - [ ] Stream prompts sent to agents (human readable)
  - [ ] Stream agent responses
  - [ ] Color-code: prompts, responses, errors
  - [ ] Auto-scroll with manual override

- [ ] **10.7** Implement keyboard controls
  - [ ] `p` - Pause: set flag, wait for current agent to complete
  - [ ] `i` - Interrupt: kill current agent process, save state
  - [ ] `q` - Quit: same as interrupt
  - [ ] Arrow keys - scroll stream view

- [ ] **10.8** Wire TUI into Manager
  - [ ] Manager sends events to TUI via std::sync::mpsc channel
  - [ ] TUI sends commands to Manager via std::sync::mpsc channel
  - [ ] Manager runs in separate thread, TUI owns main thread
  - [ ] Events: TaskStarted, AgentOutput, BuildResult, TaskCompleted
  - [ ] Commands: Pause, Interrupt, Quit

- [ ] **10.9** Verify TUI
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes
  - [ ] Manual test with mock tasks

---

## Phase 11: Polish

- [ ] **11.1** Add --dry-run mode
  - [ ] Print what would be done
  - [ ] Don't spawn agents or modify state

- [ ] **11.2** Add --verbose mode
  - [ ] Print agent prompts
  - [ ] Print agent responses
  - [ ] Detailed logging

- [ ] **11.3** Final verification
  - [ ] `cargo build --release` passes
  - [ ] `cargo clippy` passes
  - [ ] `cargo test` passes
  - [ ] Manual end-to-end test
