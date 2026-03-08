Fix: Agent spawn failures and silent phase completion

## Objective

Fix two related bugs discovered during real-world usage (bwk project):
1. Agent prompts passed via `-p` CLI arg exceed Linux ARG_MAX, causing spawn failure
2. Spawn failures leave tasks as InProgress, causing the manager loop to force-complete unimplemented phases

## Bug Details

### Bug 1: Argument list too long (os error 7)

- **Observed**: `failed to spawn agent: Argument list too long (os error 7)` when PLAN agent produces a detailed plan that gets embedded into the IMPLEM/REVIEW prompt
- **Expected**: Agent should spawn successfully regardless of prompt size
- **Location**: `src/agent/mod.rs:104-123` — prompt passed as `-p` CLI argument

### Root Cause

The prompt is passed as a command-line argument via `-p &prompt_owned` to `Command::new("claude").args(...)`. When the prompt combines agent templates (10-20KB), task instructions (1-5KB each), and git diffs (50-200KB+), the total exceeds Linux's ARG_MAX limit (~128KB-2MB), causing `execve()` to fail with E2BIG.

### Bug 2: Silent failure with forced phase completion

- **Observed**: After spawn failure, the manager re-selects the same phase, finds "no pending tasks", and forces completion — marking unimplemented phases as done
- **Expected**: Execution should stop (or at minimum, tasks should revert to Pending)
- **Location**: `src/manager/mod.rs:687-690` (tasks marked InProgress before spawn) and `src/manager/mod.rs:294-309` (error swallowed in run loop)

### Root Cause

Two issues compound:
1. At line 687-690, all tasks are marked `InProgress` BEFORE the agent spawn attempt. When spawn fails, tasks remain InProgress and are never reverted to Pending.
2. At line 294-309, the run loop catches the phase error with `// Don't propagate the error; continue with next phase`. It loops back and re-selects the same phase.
3. At line 620-642, `pending_tasks_in_phase()` finds no Pending tasks (they're InProgress), hits the "no pending tasks" branch, and forces completion.

Result: Phase-1 was marked complete without implementation. Phase-2 was marked complete without review.

## Fix Steps

### Task 1: Pass prompt via stdin

1. **Modify `spawn()` in `src/agent/mod.rs:98-137`**
   - Remove `-p` and `&prompt_owned` from `.args()`
   - Add `.stdin(Stdio::piped())` to the Command builder
   - After `.spawn()`, write the prompt to the child's stdin and close it
   - Keep the existing thread-based wait logic unchanged

2. **Modify `spawn_with_timeout()` in `src/agent/mod.rs:140-200`** (if it also uses `-p`)
   - Apply same stdin pattern

3. **Verify claude CLI supports stdin input**
   - `claude` reads from stdin when no `-p` flag is given and stdin is not a tty
   - This is the standard pipe pattern: `echo "prompt" | claude --output-format json`

### Task 2: Revert task status on spawn failure + stop on phase error

1. **Wrap spawn in try-catch and revert tasks** (`src/manager/mod.rs:~775-830`)
   - After marking tasks InProgress (line 687-690) and before/after agent spawn
   - If spawn fails (or any error occurs after tasks are marked InProgress), revert all tasks back to Pending
   - Pattern:
     ```rust
     // After marking tasks InProgress...
     let result = self.do_phase_work(...);
     if result.is_err() {
         // Revert all tasks to Pending
         for task in &pending_tasks {
             let _ = self.state.mark_task_status(&task.id, TaskStatus::Pending);
         }
     }
     result
     ```

2. **Stop execution on phase error** (`src/manager/mod.rs:294-309`)
   - Change the error handling in the run loop from swallowing errors to propagating them
   - Replace `// Don't propagate the error; continue with next phase` with actual error propagation
   - Or at minimum: log the error, save state, and break the loop instead of continuing
   - The current behavior of silently continuing is dangerous — it allows cascading failures

3. **Remove or guard the "forcing completion" logic** (`src/manager/mod.rs:627-642`)
   - The force-complete path should NOT trigger when tasks are InProgress
   - Add a check: if any tasks in the phase are InProgress, do NOT force complete
   - Only force complete when all tasks are either Completed or Deferred (legitimate terminal states)

## Files to Read

- `src/agent/mod.rs` - Current spawn implementation with `-p` flag
- `src/manager/mod.rs:280-320` - Run loop error handling
- `src/manager/mod.rs:615-700` - execute_phase entry, task status transitions
- `src/state/tasks.rs` - TaskStatus enum, pending_tasks_in_phase()

## Files to Modify

- `src/agent/mod.rs:98-200` - Switch from `-p` arg to stdin pipe for both spawn methods
- `src/manager/mod.rs:294-309` - Stop swallowing phase errors in run loop
- `src/manager/mod.rs:627-642` - Guard force-completion against InProgress tasks
- `src/manager/mod.rs:687-690` - Add error recovery to revert InProgress tasks on failure

## Verification

- [ ] `cargo build` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] Manual test: prompt larger than 128KB spawns successfully via stdin
- [ ] Manual test: simulated spawn failure reverts tasks to Pending
- [ ] Manual test: phase error stops the run loop instead of continuing

## Reviewer Criteria

**Must check:**
- [ ] `-p` flag completely removed from all spawn paths
- [ ] stdin is properly closed after writing (so claude doesn't hang waiting for input)
- [ ] Tasks are reverted to Pending on ANY error after being marked InProgress
- [ ] Run loop stops (or at least doesn't force-complete) on phase errors
- [ ] Force-completion only triggers for legitimate terminal states (Completed/Deferred)
- [ ] No regressions in normal (successful) execution path

**May skip:**
- [ ] Performance benchmarking of stdin vs -p (stdin is standard, no concern)
