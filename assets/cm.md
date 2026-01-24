---
name: cm
description: Interactive wizard for setting up cm (Claude Code Manager) projects (project)
user-invocable: true
---

# CM Project Wizard

This skill guides users through creating a complete cm (Claude Code Manager) project setup. The wizard collects information through a conversational flow and then generates all necessary artifacts in the `.cm/` directory.

## Important: Interactive Flow

You MUST follow this wizard flow step by step. Do NOT skip steps or generate files until you have gathered all the required information and received user confirmation.

---

## Step 1: Project Definition

### Step 1.1: Project Name

**Ask the user:**

> What is the **project name**?
>
> (A short identifier like `my-app`, `api-server`, `data-pipeline`)

Wait for the user's response before proceeding.

### Step 1.2: Project Description

**Ask the user:**

> What does this project do? (one sentence)

Wait for the user's response before proceeding.

### Step 1.3: Project Goal

**Ask the user:**

> What is the **overall goal**?
>
> (What problem does this solve or what does it accomplish?)

Wait for the user's response before proceeding.

---

## Step 2: Scope and Goals

### Step 2.1: Success Criteria

**Ask the user:**

> How will we know when the project is **complete**?
>
> (List 2-4 measurable success criteria)

Wait for the user's response before proceeding.

### Step 2.2: Key Features

**Ask the user:**

> What are the **key features** or capabilities?
>
> (List the main things this project should do)

Wait for the user's response before proceeding.

### Step 2.3: Constraints

**Ask the user:**

> Are there any **constraints**?
>
> (Technical constraints, deadlines, limitations - or "none")

Wait for the user's response before proceeding.

### Step 2.4: Out of Scope

**Ask the user:**

> What should we explicitly **NOT include**?
>
> (Things to avoid or defer - or "nothing specific")

Wait for the user's response before proceeding.

---

## Step 3: Reference Implementation (Optional)

**Ask the user:**

> Do you have any reference implementations or existing code to analyze?
>
> This could be:
> - An existing codebase in this repo to follow patterns from
> - A library or framework you want to integrate with
> - Documentation or specifications to follow
> - Similar projects to use as inspiration
>
> If yes, please provide file paths or URLs. If no, just say "skip" or "none".

If the user provides references:
- Analyze the code structure and patterns
- Identify coding conventions and styles
- Note any architectural patterns to follow
- Extract relevant type definitions or interfaces

Wait for the user's response before proceeding.

---

## Step 4: Phase Breakdown

**Ask the user:**

> Let's break this down into phases. What major phases do you see?
>
> Consider phases like:
> - Setup/scaffolding
> - Core implementation
> - Features/modules
> - Testing
> - Integration
> - Documentation
>
> For each phase, briefly describe:
> - What gets built
> - Dependencies on other phases

If the user needs help, suggest a reasonable phase breakdown based on the project description.

Wait for the user's response before proceeding.

---

## Step 5: Confirmation

**Present a summary to the user:**

> ## Project Summary
>
> **Project:** [name]
> **Description:** [description]
>
> **Goals:**
> - [goal 1]
> - [goal 2]
>
> **Phases:**
> 1. [Phase 1 name] - [brief description]
> 2. [Phase 2 name] - [brief description]
> ...
>
> **Files to generate:**
> - `.cm/PLAN.md` - High-level project plan
> - `.cm/ROADMAP.md` - Detailed checklist with checkboxes
> - `.cm/tasks.json` - Machine-readable task definitions
> - `.cm/LOG.md` - Execution log (empty template)
>
> Does this look correct? Reply "yes" to generate the files, or provide corrections.

Wait for explicit user confirmation before generating files.

---

## Step 6: Generate Artifacts

Once confirmed, generate all files in the `.cm/` directory:

1. Create the `.cm/` directory if it doesn't exist
2. Generate `PLAN.md` using the PLAN.md Template below
3. Generate `roadmap.json` using the roadmap.json Schema below
4. Generate `ROADMAP.md` from roadmap.json (or use template for initial creation)
5. Generate `tasks.json` using the tasks.json Schema below
6. Generate `LOG.md` as empty template (will be regenerated from tasks.json log_records)

**Note:** JSON files (tasks.json, roadmap.json) are the source of truth. Markdown files (LOG.md, ROADMAP.md) can be regenerated from JSON at any time using `cm --regenerate`.

After generation, inform the user:

> CM project initialized successfully!
>
> Created files:
> - `.cm/PLAN.md` - Review and refine the high-level plan
> - `.cm/roadmap.json` - Source of truth for roadmap progress
> - `.cm/ROADMAP.md` - Human-readable roadmap (generated from roadmap.json)
> - `.cm/tasks.json` - Used by cm to orchestrate agents
> - `.cm/LOG.md` - Execution logs (generated from tasks.json log_records)
>
> Proceeding to validation...

---

## Step 7: Validate Generated Files

After generating the files, run validation to ensure all JSON files are correct:

```bash
cm --sanity-check
```

Check the output:
- If validation **passes**: Inform the user and proceed to Step 8
- If validation **fails**:
  1. Review the error messages
  2. Fix the issues in the JSON files (tasks.json or roadmap.json)
  3. Re-run `cm --sanity-check`
  4. Repeat until all errors are resolved

**Important:** Do NOT proceed to the next step until validation passes. Common issues include:
- Invalid cross-references (roadmap_item_id pointing to non-existent item)
- Duplicate IDs in tasks or roadmap items
- Missing required fields

---

## Step 8: Git Configuration

**Ask the user:**

> Should I add `.cm/` to `.gitignore`?
>
> - **No (default)**: Keep `.cm/` tracked in git for collaboration and history
> - **Yes**: Add `.cm/` to `.gitignore` to keep project files local only
>
> Recommendation: Keep it tracked unless you have a specific reason to exclude it.

If the user chooses "yes":
1. Create or update `.gitignore`
2. Add `.cm/` on a new line

---

## Step 9: Generate Commit Message

Generate a commit message for the initial setup:

```
cm: Initialize [project-name] with [N] phases and [M] tasks
```

**Show the user:**

> Proposed commit message:
> ```
> cm: Initialize [project-name] with [N] phases and [M] tasks
> ```
>
> Would you like to use this message, or provide your own?

Wait for user to confirm or provide alternative.

---

## Step 10: Commit Changes

**Ask the user:**

> Ready to commit the `.cm/` directory with the message:
> ```
> [commit message]
> ```
>
> Proceed with commit? (yes/no)

If **yes**:
1. Stage the `.cm/` directory: `git add .cm/`
2. Commit with the message: `git commit -m "[message]"`
3. Inform user: "Committed successfully!"

If **no**:
> No problem! You can commit manually later with:
> ```bash
> git add .cm/
> git commit -m "cm: Initialize [project-name]"
> ```

---

## Templates

### PLAN.md Template

```markdown
# [Project Name]

> [One-sentence description]

## Overview

[2-3 paragraph overview explaining what this project does, why it exists, and the high-level approach]

## Goals

- [Primary goal]
- [Secondary goal]
- [Additional goals...]

## Success Criteria

- [ ] [Criterion 1]
- [ ] [Criterion 2]
- [ ] [Criterion 3]

## Architecture

[Describe the high-level architecture, main components, and how they interact]

### Components

1. **[Component 1]** - [Purpose and responsibility]
2. **[Component 2]** - [Purpose and responsibility]
3. **[Component 3]** - [Purpose and responsibility]

### Data Flow

[Describe how data flows through the system]

## Modules

### [Module 1 Name]

**Purpose:** [What this module does]

**Key files:**
- `path/to/file1.rs` - [Description]
- `path/to/file2.rs` - [Description]

**Dependencies:** [What this module depends on]

### [Module 2 Name]

[Same structure as above]

## Phases

### Phase 1: [Name]

**Goal:** [What this phase accomplishes]

**Tasks:**
- [Task 1]
- [Task 2]

**Deliverables:**
- [Deliverable 1]
- [Deliverable 2]

### Phase 2: [Name]

[Same structure as above]

## Technical Decisions

### [Decision 1]

**Context:** [Why this decision was needed]
**Decision:** [What was decided]
**Rationale:** [Why this choice was made]

## Out of Scope

- [Item 1]
- [Item 2]

## References

- [Reference 1]
- [Reference 2]
```

### ROADMAP.md Template

```markdown
# [Project Name] - Roadmap

This document tracks implementation progress. Check off items as they are completed.

## Phase 1: [Phase Name]

Status: [ ] Not Started / [ ] In Progress / [ ] Complete

### [Section 1]

- [ ] [Task 1.1] - [Brief description]
- [ ] [Task 1.2] - [Brief description]
  - [ ] [Subtask 1.2.1]
  - [ ] [Subtask 1.2.2]
- [ ] [Task 1.3] - [Brief description]

### [Section 2]

- [ ] [Task 2.1] - [Brief description]
- [ ] [Task 2.2] - [Brief description]

---

## Phase 2: [Phase Name]

Status: [ ] Not Started / [ ] In Progress / [ ] Complete

### [Section 1]

- [ ] [Task 1] - [Brief description]
- [ ] [Task 2] - [Brief description]

---

## Phase 3: [Phase Name]

[Continue pattern...]

---

## Summary

| Phase | Status | Progress |
|-------|--------|----------|
| Phase 1: [Name] | Not Started | 0/X |
| Phase 2: [Name] | Not Started | 0/X |
| Phase 3: [Name] | Not Started | 0/X |
| **Total** | | 0/X |
```

### tasks.json Schema

The `tasks.json` file follows this schema (based on `src/state/tasks.rs`):

```json
{
  "version": "1.0.0",
  "project": {
    "name": "project-name",
    "description": "Project description",
    "created_at": "2024-01-15T10:30:00Z"
  },
  "global_context": {
    "plan_summary": "High-level summary of what this project does and how"
  },
  "phases": [
    {
      "id": "phase-1",
      "name": "Phase 1 Name",
      "status": "pending",
      "tasks": [
        {
          "id": "phase-1.task-1",
          "name": "Task Name",
          "type": "implement",
          "status": "pending",
          "depends_on": [],
          "context": {
            "files_to_read": ["src/relevant/file.rs"],
            "code_style_excerpt": "Relevant style guidelines if any"
          },
          "instructions": "Detailed instructions for the agent explaining exactly what to implement, including:\n- Specific requirements\n- Expected behavior\n- Edge cases to handle\n- Files to create or modify"
        }
      ]
    }
  ],
  "current_phase": null,
  "current_task": null,
  "agent_history": []
}
```

#### Field Definitions

**TasksState (root)**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| version | string | Yes | Schema version (use "1.0.0") |
| project | Project | Yes | Project metadata |
| global_context | GlobalContext | No | Shared context for all tasks |
| phases | Phase[] | Yes | List of project phases |
| current_phase | string | No | ID of active phase |
| current_task | string | No | ID of active task |
| agent_history | AgentInvocation[] | No | History of agent runs |
| log_records | LogRecord[] | No | Structured log records for LOG.md generation |

**Project**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | string | Yes | Project identifier |
| description | string | Yes | Project description |
| created_at | datetime | No | ISO 8601 timestamp |

**GlobalContext**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| plan_summary | string | Yes | High-level plan summary |

**Phase**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Yes | Unique phase ID (e.g., "phase-1") |
| name | string | Yes | Human-readable name |
| status | PhaseStatus | Yes | "pending", "in_progress", or "completed" |
| tasks | Task[] | Yes | Tasks in this phase |

**Task**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Yes | Unique task ID (e.g., "phase-1.task-1") |
| name | string | Yes | Human-readable name |
| type | TaskType | Yes | "implement", "review", "fix", or "test" |
| status | TaskStatus | Yes | "pending", "in_progress", "completed", or "deferred" |
| depends_on | string[] | No | IDs of tasks this depends on |
| context | TaskContext | Yes | Context for the agent |
| instructions | string | Yes | Detailed instructions |
| attempts | TaskAttempt[] | No | Execution history |
| roadmap_item_id | string | No | ID of linked roadmap item (for roadmap sync) |

**TaskContext**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| files_to_read | string[] | No | Files to read for context |
| code_style_excerpt | string | No | Relevant style guidelines |
| prior_review_issues | string[] | No | Issues from prior reviews |

#### Task ID Convention

Use the format `phase-{n}.task-{m}` for task IDs:
- `phase-1.task-1` - First task in phase 1
- `phase-1.task-2` - Second task in phase 1
- `phase-2.task-1` - First task in phase 2

For review/fix cycles, extend with a suffix:
- `phase-1.task-1.review` - Review of task 1
- `phase-1.task-1.fix` - Fix issues from review

#### Dependency Rules

- Tasks can only depend on tasks from the same phase or earlier phases
- Do not create circular dependencies
- Use dependencies to ensure proper ordering (e.g., implementation before testing)

### roadmap.json Schema

The `roadmap.json` file is the source of truth for ROADMAP.md:

```json
{
  "version": "1.0.0",
  "title": "Project Name",
  "phases": [
    {
      "id": "phase-1",
      "number": "1",
      "name": "Phase Name",
      "items": [
        {
          "id": "item-1",
          "name": "Item Name",
          "completed": false,
          "sub_items": [
            { "name": "Sub-item name", "completed": false }
          ],
          "linked_task_ids": ["phase-1.task-1"]
        }
      ]
    }
  ]
}
```

**RoadmapState (root)**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| version | string | Yes | Schema version (use "1.0.0") |
| title | string | Yes | Project title |
| phases | RoadmapPhase[] | Yes | List of phases |

**RoadmapPhase**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Yes | Unique phase ID |
| number | string | Yes | Phase number (e.g., "1", "2", "0.5") |
| name | string | Yes | Phase name |
| items | RoadmapItem[] | Yes | Items in this phase |

**RoadmapItem**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | string | Yes | Unique item ID |
| name | string | Yes | Item name |
| completed | boolean | Yes | Whether item is completed |
| sub_items | RoadmapSubItem[] | No | Sub-items |
| linked_task_ids | string[] | No | IDs of linked tasks |

**RoadmapSubItem**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | string | Yes | Sub-item name |
| completed | boolean | Yes | Whether sub-item is completed |

### LOG.md Template

```markdown
# [Project Name] - Execution Log

This file is automatically updated by cm during task execution.

## Log Format

Each entry follows this format:

```
## [YYYY-MM-DD HH:MM:SS] Task: task-id

**Status:** [started|completed|failed|deferred]
**Agent:** [agent-type]
**Duration:** [X minutes]

### Summary
[Brief summary of what was done]

### Files Changed
- `path/to/file.rs` - [created|modified|deleted]

### Commands Run
- `command 1`
- `command 2`

### Notes
[Any additional notes or issues encountered]

---
```

## Execution Log

[Entries will be appended below during execution]

---
```

---

## Example Generation

Here's an example of what the wizard should generate for a simple CLI tool project:

### Example PLAN.md

```markdown
# my-cli-tool

> A command-line tool for processing JSON files

## Overview

my-cli-tool is a Rust CLI application that reads JSON files, applies transformations, and outputs the results. It supports filtering, mapping, and aggregation operations.

## Goals

- Parse and validate JSON input files
- Apply user-specified transformations
- Output results in multiple formats (JSON, CSV, table)

## Success Criteria

- [ ] Can read JSON files from disk or stdin
- [ ] Supports at least 5 transformation operations
- [ ] Handles malformed input gracefully
- [ ] Has comprehensive test coverage

## Architecture

The tool follows a pipeline architecture:
1. Input parsing
2. Transformation application
3. Output formatting

### Components

1. **Parser** - Reads and validates JSON input
2. **Transformer** - Applies operations to data
3. **Formatter** - Outputs results in desired format

## Phases

### Phase 1: Foundation

**Goal:** Set up project structure and basic CLI

**Tasks:**
- Initialize Cargo project
- Add clap for argument parsing
- Create basic CLI structure

### Phase 2: Core Implementation

**Goal:** Implement parsing and transformation logic

**Tasks:**
- JSON parser module
- Transformation operations
- Error handling

### Phase 3: Output and Polish

**Goal:** Add output formatters and polish

**Tasks:**
- JSON formatter
- CSV formatter
- Table formatter
- Documentation
```

### Example tasks.json

```json
{
  "version": "1.0.0",
  "project": {
    "name": "my-cli-tool",
    "description": "A command-line tool for processing JSON files",
    "created_at": "2024-01-15T10:30:00Z"
  },
  "global_context": {
    "plan_summary": "Build a Rust CLI tool that reads JSON files, applies transformations (filter, map, aggregate), and outputs in multiple formats (JSON, CSV, table). Uses clap for CLI parsing and serde for JSON handling."
  },
  "phases": [
    {
      "id": "phase-1",
      "name": "Foundation",
      "status": "pending",
      "tasks": [
        {
          "id": "phase-1.task-1",
          "name": "Initialize project structure",
          "type": "implement",
          "status": "pending",
          "depends_on": [],
          "context": {
            "files_to_read": []
          },
          "instructions": "Initialize a new Rust project with Cargo:\n\n1. Create the project structure with `cargo init`\n2. Add dependencies to Cargo.toml:\n   - clap with derive feature\n   - serde with derive feature\n   - serde_json\n   - thiserror\n3. Create src/main.rs with basic clap CLI setup\n4. Create src/lib.rs exporting modules\n5. Ensure `cargo build` succeeds"
        },
        {
          "id": "phase-1.task-2",
          "name": "Create CLI argument parsing",
          "type": "implement",
          "status": "pending",
          "depends_on": ["phase-1.task-1"],
          "context": {
            "files_to_read": ["src/main.rs", "Cargo.toml"]
          },
          "instructions": "Implement CLI argument parsing using clap:\n\n1. Define Args struct with:\n   - input: Option<PathBuf> for input file (default: stdin)\n   - output: Option<PathBuf> for output file (default: stdout)\n   - format: OutputFormat enum (json, csv, table)\n   - operation: Operation subcommand\n2. Implement subcommands for: filter, map, select\n3. Add help text and examples\n4. Parse args in main() and print them for now"
        }
      ]
    },
    {
      "id": "phase-2",
      "name": "Core Implementation",
      "status": "pending",
      "tasks": [
        {
          "id": "phase-2.task-1",
          "name": "Implement JSON parser module",
          "type": "implement",
          "status": "pending",
          "depends_on": ["phase-1.task-2"],
          "context": {
            "files_to_read": ["src/lib.rs", "src/main.rs"]
          },
          "instructions": "Create the JSON parser module:\n\n1. Create src/parser.rs module\n2. Implement read_json() function that:\n   - Accepts a Read trait object\n   - Parses JSON into serde_json::Value\n   - Returns Result with custom error type\n3. Handle both JSON objects and arrays\n4. Add unit tests for valid and invalid input\n5. Export from lib.rs"
        }
      ]
    }
  ],
  "current_phase": null,
  "current_task": null,
  "agent_history": []
}
```

---

## Tips for Good Task Definitions

1. **Be specific** - Include exact file paths, function names, and requirements
2. **One task, one goal** - Each task should have a single clear objective
3. **Include context** - List files the agent should read for understanding
4. **Specify success criteria** - How will we know the task is complete?
5. **Handle dependencies** - Ensure tasks are ordered correctly
6. **Keep instructions actionable** - Use imperative language ("Create X", "Implement Y")
