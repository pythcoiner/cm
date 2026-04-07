# cm - Tasks

This document shows phase plans and task status. Generated from tasks.json.

## phase-0: Project Setup

**Status:** Complete (0/0)


---

## phase-0.5: cm init + Skills

**Status:** Complete (0/0)


---

## phase-1: State Types

**Status:** Complete (0/0)


---

## phase-2: Agent Module

**Status:** Complete (0/0)


---

## phase-3: Build Module

**Status:** Complete (0/0)


---

## phase-4: Log Module

**Status:** Complete (0/0)


---

## phase-5: Manager Module

**Status:** Complete (0/0)


---

## phase-6: Recovery Module

**Status:** Complete (0/0)


---

## phase-7: CLI Module

**Status:** Complete (0/0)


---

## phase-8: Skills Module

**Status:** Complete (0/0)


---

## phase-9: Integration Tests

**Status:** Complete (0/0)


---

## phase-10: TUI Module

**Status:** Complete (0/0)


---

## phase-11: Polish

**Status:** Complete (0/0)


---

## phase-12: Configuration

**Status:** Complete (0/0)


---

## phase-13: TUI Integration

**Status:** Complete (0/0)


---

## phase-14: Signal Handling

**Status:** Complete (0/0)


---

## phase-15: JSON Source of Truth

**Status:** Complete (0/0)


---

## phase-16: Sanity Check Workflow

**Status:** Complete (0/0)


---

## phase-18: Regenerate MD After Skills

**Status:** Complete (0/0)


---

## phase-23: Detailed File Logging

**Status:** Complete (0/0)


---

## phase-26: Skills to Commands Migration

**Status:** Complete (0/0)


---

## phase-27: Add /run Command

**Status:** Complete (1/1)

### Tasks

- [x] **phase-27.feat-run-command.impl-1**: Add /run command for cm orchestration - Add the /run command that provides main agent orchestration instructions:

---

## phase-28: Remove LOG.md Generation

**Status:** Complete (1/1)

### Tasks

- [x] **phase-28.feat-remove-log-md.impl-1**: Remove LOG.md generation entirely - Remove LOG.md generation from the codebase entirely:

---

## phase-29: Add --reset Flag

**Status:** Complete (1/1)

### Tasks

- [x] **phase-29.feat-reset-flag.impl-1**: Add --reset flag to reset a phase - Add a --reset <PHASE_ID> CLI flag that resets a phase to pending status:

---

## phase-30: Add /cm Prerequisites Check

**Status:** Complete (1/1)

### Tasks

- [x] **phase-30.feat-cm-prereq.impl-1**: Add Step 0 prerequisites check to /cm wizard - Add a Step 0 prerequisites check to the /cm wizard in `assets/cm.md`:

---

## phase-31: Add .cm/logs/ to .gitignore

**Status:** Complete (1/1)

### Tasks

- [x] **phase-31.feat-gitignore.impl-1**: Add .cm/logs/ to root .gitignore during cm init - # Plan: Add .cm/logs/ to .gitignore during cm init

---

## phase-32: Add Phase Selection by Number

**Status:** Complete (1/1)

### Tasks

- [x] **phase-32.feat-phase-select.impl-1**: Add [p]hase <#...> option to run specific phases by number - # Plan: Add Phase Selection by Number

---

## phase-33: Add PLAN Agent Before IMPLEM

**Status:** Complete (1/1)

### Tasks

- [x] **phase-33.feat-plan-agent.impl-1**: Add PLAN agent step before IMPLEM with conditional detailed planning - # Plan: Add PLAN Agent Before IMPLEM

---

## phase-34: Add /split Command + Extend /end

**Status:** Complete (2/2)

### Tasks

- [x] **phase-34.feat-split-command.impl-1**: Add /split command to refine PLAN.md into detailed phases - # Plan: Add /split Command + Extend /end for /cm Sessions
- [x] **phase-34.feat-end-cm-session.impl-2**: Extend /end to support /cm session finalization - # Plan: Add /split Command + Extend /end for /cm Sessions

---

## phase-35: Remove TUI Module

**Status:** Complete (1/1)

### Tasks

- [x] **phase-35.feat-remove-tui.impl-1**: Remove TUI and make daemon mode default - # Plan: Remove TUI and Make Daemon Mode Default

---

## phase-36: Remove Unused MANAGER.md

**Status:** Complete (1/1)

### Tasks

- [x] **phase-36.feat-remove-manager.impl-1**: Remove unused MANAGER.md template and related code - # Plan: Remove Unused MANAGER.md

---

## phase-37: Add Phase Range Selection

**Status:** Complete (1/1)

### Tasks

- [x] **phase-37.feat-phase-range.impl-1**: Add range-based phase selection (e.g., p 3-6) - # Plan: Add Range-Based Phase Selection

---

## phase-38: Add Model Selection Flag

**Status:** Complete (1/1)

### Tasks

- [x] **phase-38.feat-model-flag.impl-1**: Add --model flag with sonnet/opus shorthand - # Plan: Add Shorthand Model Selection (sonnet/opus)

---

## phase-39: Fix Agent Spawn Failures

**Status:** Complete (3/3)

### Plan

Fix two bugs: (1) prompt passed via -p CLI arg exceeds ARG_MAX — switch to stdin, (2) spawn failures leave tasks InProgress causing forced completion of unimplemented phases — revert tasks on error and stop execution.

### Tasks

- [x] **phase-39.fix-stdin-prompt.impl-1**: Fix: Pass agent prompt via stdin instead of -p CLI arg - Fix: Agent spawn failures and silent phase completion
- [x] **phase-39.fix-error-handling.impl-2**: Fix: Revert task status on spawn failure and stop on phase error - Fix: Agent spawn failures and silent phase completion
- [x] **phase-39.review**: Review agent spawn fix and error handling - Fix: Agent spawn failures and silent phase completion

---

## phase-40: Post-Run Review Agent

**Status:** Pending (0/6)

### Plan

After every cm run (normal, Ctrl-C, or crash), spawn a review agent that inspects all logs from phases touched during the run, prints a markdown report to stdout verbatim, saves it to .cm/reports/, and exits non-zero if issues are found.

### Tasks

- [ ] **phase-40.feat-run-review.impl-1**: Add run-tracking state (run_started_at, touched_phases) - # Phase 40: Post-Run Review Agent
- [ ] **phase-40.feat-run-review.impl-2**: Create src/review/mod.rs log gathering module - # Phase 40: Post-Run Review Agent
- [ ] **phase-40.feat-run-review.impl-3**: Add RunReview agent kind, prompt, and response parser - # Phase 40: Post-Run Review Agent
- [ ] **phase-40.feat-run-review.impl-4**: Wire run_post_run_review into orchestrator and propagate exit code - # Phase 40: Post-Run Review Agent
- [ ] **phase-40.feat-run-review.test-1**: Tests for log gathering and trailing-JSON parser - # Phase 40: Post-Run Review Agent
- [ ] **phase-40.feat-run-review.review**: Review post-run review feature - # Phase 40: Post-Run Review Agent

---

## Summary

| Phase | Status | Tasks | Completed |
|-------|--------|-------|----------:|
| phase-0: Project Setup | Complete | 0 | 0 |
| phase-0.5: cm init + Skills | Complete | 0 | 0 |
| phase-1: State Types | Complete | 0 | 0 |
| phase-2: Agent Module | Complete | 0 | 0 |
| phase-3: Build Module | Complete | 0 | 0 |
| phase-4: Log Module | Complete | 0 | 0 |
| phase-5: Manager Module | Complete | 0 | 0 |
| phase-6: Recovery Module | Complete | 0 | 0 |
| phase-7: CLI Module | Complete | 0 | 0 |
| phase-8: Skills Module | Complete | 0 | 0 |
| phase-9: Integration Tests | Complete | 0 | 0 |
| phase-10: TUI Module | Complete | 0 | 0 |
| phase-11: Polish | Complete | 0 | 0 |
| phase-12: Configuration | Complete | 0 | 0 |
| phase-13: TUI Integration | Complete | 0 | 0 |
| phase-14: Signal Handling | Complete | 0 | 0 |
| phase-15: JSON Source of Truth | Complete | 0 | 0 |
| phase-16: Sanity Check Workflow | Complete | 0 | 0 |
| phase-18: Regenerate MD After Skills | Complete | 0 | 0 |
| phase-23: Detailed File Logging | Complete | 0 | 0 |
| phase-26: Skills to Commands Migration | Complete | 0 | 0 |
| phase-27: Add /run Command | Complete | 1 | 1 |
| phase-28: Remove LOG.md Generation | Complete | 1 | 1 |
| phase-29: Add --reset Flag | Complete | 1 | 1 |
| phase-30: Add /cm Prerequisites Check | Complete | 1 | 1 |
| phase-31: Add .cm/logs/ to .gitignore | Complete | 1 | 1 |
| phase-32: Add Phase Selection by Number | Complete | 1 | 1 |
| phase-33: Add PLAN Agent Before IMPLEM | Complete | 1 | 1 |
| phase-34: Add /split Command + Extend... | Complete | 2 | 2 |
| phase-35: Remove TUI Module | Complete | 1 | 1 |
| phase-36: Remove Unused MANAGER.md | Complete | 1 | 1 |
| phase-37: Add Phase Range Selection | Complete | 1 | 1 |
| phase-38: Add Model Selection Flag | Complete | 1 | 1 |
| phase-39: Fix Agent Spawn Failures | Complete | 3 | 3 |
| phase-40: Post-Run Review Agent | Pending | 6 | 0 |
| **Total** | | **22** | **16** |
