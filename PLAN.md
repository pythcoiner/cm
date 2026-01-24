# Claude Code Manager (cm) - Implementation Plan

## Overview

A Rust CLI tool that automates coordination between Claude agents for software development tasks. The manager mediates all agent communication - agents never interact directly with each other.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    cm (Rust binary)                     │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   State     │  │   Agent     │  │   Build         │ │
│  │   Manager   │  │   Spawner   │  │   Verifier      │ │
│  │ tasks.json  │  │ claude -p   │  │ cargo/git       │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
└──────────────────────────┬──────────────────────────────┘
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
│ IMPLEM Agent│     │REVIEW Agent │     │  FIX Agent  │
│ Task ctx    │     │ Same ctx    │     │ Ctx+issues  │
│ only        │     │ as IMPLEM   │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

## Core Principles

1. **Context Isolation**: IMPLEM/REVIEW agents receive ONLY task-specific context, never global knowledge
2. **Manager as Mediator**: All communication via manager, no direct agent interaction
3. **State in tasks.json**: Complete history for auditing and crash recovery
4. **Review Loop**: IMPLEM → REVIEW → (FIX → REVIEW)* until approved (max 5 cycles, then defer)
5. **Task Dependencies**: Tasks track `depends_on` which main agent can modify at runtime
6. **Sequential Execution**: Tasks run one at a time for predictability

---

## Development Workflow (Agent-Driven)

This project is built using the agent-driven workflow. The MAIN AGENT coordinates sub-agents.

### Phase Workflow

For each phase:

```
┌─────────────────────────────────────────────────────────┐
│ MAIN AGENT                                              │
│ 1. READ current state from ROADMAP.md and LOG.md       │
│ 2. SPAWN implementation sub-agent with detailed prompt  │
│ 3. WAIT for implementation to complete                  │
│ 4. SPAWN review sub-agent (FRESH context!)              │
│ 5. IF review finds issues: SPAWN fix, then re-review    │
│ 6. REPEAT until review APPROVED (max 5 cycles)          │
│ 7. UPDATE ROADMAP.md - check off completed items        │
│ 8. UPDATE LOG.md - add detailed log entry               │
│ 9. GIT COMMIT                                           │
└─────────────────────────────────────────────────────────┘
```

### Sub-Agent Types

| Agent Type | When Created | Context Given |
|-----------|--------------|---------------|
| **IMPLEM** | For each task | Task instructions + files to read + CODE_STYLE.md |
| **REVIEW** | After IMPLEM | Same files as IMPLEM + expected output |
| **FIX** | If review finds issues | Same as IMPLEM + review issues |

### Critical Rules

1. **Fresh agent per task** - Never reuse agent context across phases
2. **No global knowledge** - Sub-agents only know their task context
3. **Documentation is state** - ROADMAP.md checkboxes track progress
4. **Append-only LOG.md** - Every action is logged with details

---

## CLI Interface

```
cm               # Execute tasks until complete
cm --continue    # Resume from interrupted state
cm --step        # Execute one task, then pause
cm --status      # Show progress
cm --validate    # Validate tasks.json schema
```

## Terminal UI (ratatui)

Split-view TUI showing real-time progress:

```
┌─ Tasks ──────────────────────┬─ Agent Stream ─────────────────────────────┐
│                              │                                            │
│ Phase 3: State Types         │ [IMPLEM] Task 3.2: Create Task struct      │
│   ✓ 3.1 Create mod.rs        │                                            │
│   ⠋ 3.2 Create Task struct   │ > Reading CODE_STYLE.md...                 │
│   ○ 3.3 Create TaskAttempt   │ > Creating src/state/tasks.rs              │
│   ○ 3.4 Create AgentInvoc    │ > Implementing Task struct with fields:    │
│                              │   - id: TaskId                             │
│ Phase 4: Agent Module        │   - name: String                           │
│   ○ 4.1 Create mod.rs        │   - status: TaskStatus                     │
│   ○ 4.2 AgentSpawner         │   - depends_on: Vec<TaskId>                │
│   ...                        │ > Running cargo build...                   │
│                              │ > Build: PASS                              │
│                              │                                            │
├──────────────────────────────┴────────────────────────────────────────────┤
│ [p] Pause (wait for agent)  [i] Interrupt agent  [q] Quit                 │
└───────────────────────────────────────────────────────────────────────────┘
```

**Controls:**
- `p` - Pause: finish current agent, then stop
- `i` - Interrupt: kill current agent immediately, save state
- `q` - Quit: same as interrupt

**Verbose by default** - shows full prompt/response stream in right pane

## Init Skill (Claude Code)

Location: `.claude/skills/cm.md`

The `/cm` skill is an interactive wizard that helps users generate:
- `.cm/PLAN.md` - Overall plan
- `.cm/ROADMAP.md` - Task checklist
- `.cm/tasks.json` - Initial state with all tasks
- `.cm/LOG.md` - Template

**Flow:**
1. User runs `/cm` in Claude Code
2. Skill asks questions about what to build
3. Skill generates all artifacts in `.cm/`
4. User exits Claude Code
5. User runs `cm` binary to execute the plan autonomously

---

## Key Data Structures

### tasks.json Schema

```json
{
  "version": "1.0.0",
  "project": { "name": "...", "description": "..." },
  "global_context": { "plan_summary": "..." },
  "current_phase": "phase-1",
  "current_task": "phase-1.task-2",
  "phases": [
    {
      "id": "phase-1",
      "name": "Setup",
      "status": "in_progress",
      "tasks": [
        {
          "id": "phase-1.task-1",
          "type": "implement",
          "status": "completed",
          "depends_on": [],
          "context": {
            "files_to_read": ["src/lib.rs"],
            "code_style_excerpt": "..."
          },
          "instructions": "...",
          "attempts": [...]
        }
      ]
    }
  ],
  "agent_history": [...]
}
```

### Task Statuses

- `pending` - Not started, dependencies may not be met
- `blocked` - Dependencies not completed (computed, not stored)
- `in_progress` - Currently being worked on
- `completed` - Successfully finished
- `deferred` - Failed after 5 cycles, skipped for now

### Task Dependencies

- Each task has `depends_on: Vec<TaskId>`
- Main agent can modify dependencies at runtime
- A task is BLOCKED if any dependency is not COMPLETED
- Manager skips blocked tasks, continues with runnable ones

---

## Module Structure

```
cm/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry, clap
│   ├── lib.rs            # Library exports
│   │
│   ├── manager/
│   │   ├── mod.rs        # Main orchestration loop
│   │   ├── state.rs      # State machine
│   │   └── recovery.rs   # Crash recovery
│   │
│   ├── state/
│   │   ├── mod.rs        # tasks.json handling
│   │   ├── tasks.rs      # Task/Phase structs
│   │   └── schema.rs     # JSON validation
│   │
│   ├── agent/
│   │   ├── mod.rs        # Spawn `claude -p --output-format json`
│   │   ├── prompt.rs     # Build prompts (context isolation)
│   │   └── response.rs   # Parse JSON responses
│   │
│   ├── build/
│   │   ├── mod.rs        # Build verification
│   │   ├── cargo.rs      # cargo build/clippy/test
│   │   └── git.rs        # git commit/status
│   │
│   ├── log/
│   │   └── mod.rs        # LOG.md management
│   │
│   ├── tui/
│   │   ├── mod.rs        # TUI app, terminal setup
│   │   ├── layout.rs     # Split view layout
│   │   └── widgets.rs    # TaskList, Stream, Controls
│   │
│   └── cli/
│       └── mod.rs        # CLI args parsing
│
└── tests/
    └── integration.rs
```

---

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
log = "0.4"
env_logger = "0.11"

# TUI
ratatui = "0.26"
crossterm = "0.27"

# No tokio - use std::thread, std::sync::mpsc, std::process
```

---

## Design Decisions

### Error Handling
- Use `thiserror` for error enums, never `anyhow`
- Each module has its own error type
- Errors propagate with context

### Persistence
- tasks.json is the single source of truth
- Checkpoint before mutations for recovery
- LOG.md is append-only audit trail

### Agent Spawning
- Use `claude -p "prompt" --output-format json`
- Parse structured JSON responses
- Agents return output, they do NOT edit any files directly
- cm (manager) is responsible for all file edits

### Logging
- cm owns LOG.md - agents never touch it
- cm logs: prompts sent, responses received, build results, review verdicts
- All logging happens in the manager, not in agents

### Review Cycle
- Max 5 IMPLEM→REVIEW cycles per task
- After 5 failures, mark task as DEFERRED
- Continue with next non-blocked task
