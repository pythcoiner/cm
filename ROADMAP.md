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

- [ ] **1.1** Create src/state/mod.rs
  - [ ] Module declarations
  - [ ] StateError enum with thiserror
  - [ ] load_state() function
  - [ ] save_state() function

- [ ] **1.2** Create src/state/tasks.rs with core types
  - [ ] TasksState struct (version, project, phases, current_phase, current_task, agent_history)
  - [ ] Project struct (name, description, created_at)
  - [ ] GlobalContext struct (plan_summary)
  - [ ] Phase struct (id, name, status, tasks)
  - [ ] PhaseStatus enum (Pending, InProgress, Completed)

- [ ] **1.3** Create Task type
  - [ ] Task struct (id, name, task_type, status, depends_on, context, instructions, attempts)
  - [ ] TaskType enum (Implement, Review, Fix, Test)
  - [ ] TaskStatus enum (Pending, InProgress, Completed, Deferred)
  - [ ] TaskContext struct (files_to_read, code_style_excerpt, prior_review_issues)

- [ ] **1.4** Create TaskAttempt and related types
  - [ ] TaskAttempt struct (attempt_number, agent_id, started_at, completed_at, status, response)
  - [ ] AttemptStatus enum (Success, Failed, Timeout)
  - [ ] AgentResponse struct (files_created, files_modified, commands_run, raw_response)
  - [ ] ReviewResult struct (verdict, issues)
  - [ ] ReviewIssue struct (id, severity, location, problem, suggested_fix, resolved)
  - [ ] Severity enum (Critical, High, Medium, Low)
  - [ ] Verdict enum (Approved, NeedsFixes)

- [ ] **1.5** Create AgentInvocation type
  - [ ] AgentInvocation struct (id, task_id, agent_type, started_at, completed_at, exit_status)
  - [ ] AgentType enum (Main, Implem, Review, Fix)

- [ ] **1.6** Implement Serialize/Deserialize for all types
  - [ ] Derive serde traits
  - [ ] Use rename_all = "snake_case" for enums
  - [ ] DateTime<Utc> serialization

- [ ] **1.7** Add helper methods to TasksState
  - [ ] current_phase() -> Option<&Phase>
  - [ ] current_task() -> Option<&Task>
  - [ ] next_runnable_task() -> Option<&Task>
  - [ ] is_task_blocked(task_id) -> bool
  - [ ] mark_task_status(task_id, status)

- [ ] **1.8** Verify state module
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes
  - [ ] Unit tests for serialization round-trip

---

## Phase 2: Agent Module

- [ ] **2.1** Create src/agent/mod.rs
  - [ ] Module declarations
  - [ ] AgentError enum with thiserror
  - [ ] AgentSpawner struct

- [ ] **2.2** Implement AgentSpawner
  - [ ] new(model, timeout) constructor
  - [ ] spawn(prompt, task_id) -> Result<AgentHandle>
  - [ ] Use std::process::Command for `claude -p`
  - [ ] Capture stdout/stderr
  - [ ] Return AgentHandle with process handle

- [ ] **2.3** Create AgentHandle type
  - [ ] AgentHandle struct (thread handle, stop_flag, task_id, started_at)
  - [ ] wait() -> Result<AgentOutput> (blocks until thread completes)
  - [ ] interrupt() -> kill child process via stop_flag
  - [ ] Use std::thread, NOT async

- [ ] **2.4** Create src/agent/prompt.rs
  - [ ] PromptBuilder struct
  - [ ] build_implem_prompt(task) -> String
  - [ ] build_review_prompt(task) -> String
  - [ ] build_fix_prompt(task, issues) -> String
  - [ ] Ensure NO global context leaks into prompts

- [ ] **2.5** Create src/agent/response.rs
  - [ ] ResponseParser struct
  - [ ] parse(raw_json) -> Result<AgentResponse>
  - [ ] Handle malformed JSON gracefully
  - [ ] Extract files_created, files_modified, commands_run

- [ ] **2.6** Verify agent module
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes
  - [ ] Unit tests for prompt building

---

## Phase 3: Build Module

- [ ] **3.1** Create src/build/mod.rs
  - [ ] Module declarations
  - [ ] BuildError enum with thiserror
  - [ ] BuildVerifier struct

- [ ] **3.2** Create src/build/cargo.rs
  - [ ] CargoRunner struct
  - [ ] build() -> Result<BuildOutput>
  - [ ] clippy() -> Result<ClippyOutput>
  - [ ] test() -> Result<TestOutput>
  - [ ] Parse cargo output for errors/warnings

- [ ] **3.3** Create src/build/git.rs
  - [ ] GitRunner struct
  - [ ] status() -> Result<GitStatus>
  - [ ] commit(message) -> Result<CommitId>
  - [ ] add(files) -> Result<()>

- [ ] **3.4** Implement BuildVerifier
  - [ ] new() constructor
  - [ ] verify_build() -> Result<()>
  - [ ] verify_clippy() -> Result<()>
  - [ ] Run build then clippy, collect all errors

- [ ] **3.5** Verify build module
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes

---

## Phase 4: Log Module

**IMPORTANT:** cm (manager) owns LOG.md - agents never touch it.

- [ ] **4.1** Create src/log/mod.rs
  - [ ] LogError enum with thiserror
  - [ ] LogManager struct

- [ ] **4.2** Implement LogManager
  - [ ] new(path) constructor
  - [ ] append_entry(entry) -> Result<()>
  - [ ] LogEntry struct (timestamp, phase, task, agent_id, action, details)

- [ ] **4.3** Implement log formatting
  - [ ] format_phase_start(phase) -> String
  - [ ] format_agent_spawn(agent_type, task_id, prompt) -> String
  - [ ] format_agent_response(response) -> String
  - [ ] format_build_result(output) -> String
  - [ ] format_review_result(verdict, issues) -> String

- [ ] **4.4** Manager calls LogManager
  - [ ] Log before spawning agent (prompt)
  - [ ] Log after agent completes (response)
  - [ ] Log build/test results
  - [ ] Log task completion/deferral

- [ ] **4.5** Verify log module
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes

---

## Phase 5: Manager Core

- [ ] **5.1** Create src/manager/mod.rs
  - [ ] Module declarations
  - [ ] ManagerError enum with thiserror
  - [ ] Manager struct

- [ ] **5.2** Implement Manager struct
  - [ ] Fields: state, config, agent_spawner, build_verifier, log_manager
  - [ ] new(config) -> Result<Self>
  - [ ] load_or_create_state() -> Result<TasksState>

- [ ] **5.3** Implement main orchestration loop
  - [ ] run() -> Result<()>
  - [ ] Loop: select_next_task() -> execute_task() -> update_state()
  - [ ] Handle all task types (Implement, Review, Fix)
  - [ ] Check dependencies before executing

- [ ] **5.4** Implement IMPLEM flow
  - [ ] Build prompt with task context only
  - [ ] Spawn agent
  - [ ] Wait for response
  - [ ] Parse response
  - [ ] Run build verification
  - [ ] If build fails, create FIX task
  - [ ] If build passes, create REVIEW task

- [ ] **5.5** Implement REVIEW flow
  - [ ] Build review prompt
  - [ ] Spawn agent
  - [ ] Parse review result
  - [ ] If APPROVED, mark task complete
  - [ ] If NEEDS_FIXES, create FIX task

- [ ] **5.6** Implement FIX flow
  - [ ] Build fix prompt with issues
  - [ ] Spawn agent
  - [ ] Parse response
  - [ ] Run build verification
  - [ ] Create REVIEW task

- [ ] **5.7** Implement cycle limiting
  - [ ] Track attempt count per task
  - [ ] After 5 cycles, mark task as DEFERRED
  - [ ] Continue with next non-blocked task

- [ ] **5.8** Create src/manager/state.rs
  - [ ] ManagerState enum (Idle, Executing, WaitingForAgent, Verifying)
  - [ ] State transition methods

- [ ] **5.9** Verify manager core
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes

---

## Phase 6: Recovery

- [ ] **6.1** Create src/manager/recovery.rs
  - [ ] RecoveryManager struct
  - [ ] RecoveryError enum

- [ ] **6.2** Implement checkpointing
  - [ ] checkpoint() -> Result<CheckpointId>
  - [ ] Save state before mutations
  - [ ] Store in .cm/checkpoints/

- [ ] **6.3** Implement restore
  - [ ] restore(checkpoint_id) -> Result<()>
  - [ ] Load state from checkpoint

- [ ] **6.4** Implement crash recovery
  - [ ] recover_from_crash() -> Result<RecoveryAction>
  - [ ] Detect incomplete task
  - [ ] Determine recovery action (retry, skip, etc.)

- [ ] **6.5** Implement graceful shutdown
  - [ ] Handle SIGINT/SIGTERM
  - [ ] Wait for current agent (with timeout)
  - [ ] Save state with interrupted marker
  - [ ] Append shutdown to LOG.md

- [ ] **6.6** Verify recovery module
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes

---

## Phase 7: CLI Commands

- [ ] **7.1** Create src/cli/mod.rs
  - [ ] Cli struct with clap derive
  - [ ] Flags: --continue, --step, --status, --validate, --verbose
  - [ ] Options: --config, --state

- [ ] **7.2** Implement main execution (default)
  - [ ] Load state from .cm/tasks.json
  - [ ] Create Manager
  - [ ] Call manager.run()

- [ ] **7.3** Implement --continue flag
  - [ ] Load state
  - [ ] Call recovery.recover_from_crash()
  - [ ] Resume execution

- [ ] **7.4** Implement --step flag
  - [ ] Execute one task only
  - [ ] Save state and exit

- [ ] **7.5** Implement --status flag
  - [ ] Load state
  - [ ] Display progress summary
  - [ ] Show current phase/task
  - [ ] Show deferred tasks

- [ ] **7.6** Implement --validate flag
  - [ ] Load tasks.json
  - [ ] Validate against schema
  - [ ] Report errors

- [ ] **7.7** Wire up main.rs
  - [ ] Parse CLI args
  - [ ] Dispatch based on flags
  - [ ] Set up logging (--verbose)

- [ ] **7.8** Verify CLI
  - [ ] `cargo build` passes
  - [ ] `cargo clippy` passes
  - [ ] `cm --help` works

---

## Phase 8: Claude Code Skill

- [ ] **8.1** Create .claude/skills/cm.md
  - [ ] Skill name and description
  - [ ] Usage: /cm

- [ ] **8.2** Define interactive wizard flow
  - [ ] Ask: What do you want to build?
  - [ ] Ask: What is the scope/goal?
  - [ ] Ask: Any reference implementation to analyze?
  - [ ] Ask: What phases do you see?
  - [ ] Confirm understanding before generating

- [ ] **8.3** Implement artifact generation
  - [ ] Generate .cm/PLAN.md from conversation
  - [ ] Generate .cm/ROADMAP.md with detailed tasks
  - [ ] Generate .cm/tasks.json with all tasks and dependencies
  - [ ] Initialize .cm/LOG.md

- [ ] **8.4** Create generation templates
  - [ ] PLAN.md template structure
  - [ ] ROADMAP.md checklist format
  - [ ] tasks.json schema with depends_on
  - [ ] LOG.md header template

---

## Phase 9: Integration Testing

- [ ] **9.1** Create tests/integration.rs
  - [ ] Test setup helpers

- [ ] **9.2** Test basic flow
  - [ ] Create minimal tasks.json
  - [ ] Run cm with mock agent
  - [ ] Verify state updates

- [ ] **9.3** Test recovery
  - [ ] Simulate crash during execution
  - [ ] Run cm continue
  - [ ] Verify resumed correctly

- [ ] **9.4** Test deferred tasks
  - [ ] Create task that fails 5 times
  - [ ] Verify marked as deferred
  - [ ] Verify next task runs

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
