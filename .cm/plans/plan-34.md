# Plan: Add /split Command + Extend /end for /cm Sessions

## Summary
1. **`/split`** - Refines PLAN.md into detailed phases (before JSON generation)
2. **`/end` extension** - Now also works after `/cm` sessions to finalize and generate tasks.json + roadmap.json

## Complete Workflow
```
/cm           → Creates high-level PLAN.md (interactive wizard)
    ↓
/split        → (optional) Refines PLAN.md into detailed phases
    ↓
/end          → Finalizes session, generates tasks.json + roadmap.json
```

## /end Behavior by Session Type
| Session | /end Action |
|---------|-------------|
| `/feat` | Add feature tasks to existing tasks.json/roadmap.json |
| `/fix`  | Add fix tasks to existing tasks.json/roadmap.json |
| `/cm`   | **NEW:** Generate tasks.json + roadmap.json from PLAN.md |
| `/cm` + `/split` | **NEW:** Generate from detailed PLAN.md |

---

# Part 1: /split Command

## Purpose
After `/cm` creates a high-level PLAN.md, `/split` refines it by:
1. Analyzing the phases described in PLAN.md
2. Breaking each phase into smaller, more detailed sub-phases
3. Updating PLAN.md with the detailed breakdown
4. **Does NOT generate JSON** - that's `/end`'s job

## Files to Create/Modify

1. `assets/split.md` - New command wizard file
2. `src/command.rs` - Add `SPLIT_COMMAND` constant
3. `src/cli/init.rs` - Deploy split.md during `cm init`

## Implementation Steps

### 1. Create `assets/split.md`

```markdown
# Split Wizard

This skill refines an existing cm project plan by splitting high-level phases into detailed sub-phases.

## Prerequisites

Before using this wizard, ensure:
- A `.cm/` directory exists with valid project files
- `tasks.json` and `roadmap.json` are present (run `/cm` first)
- You have phases that need more detailed breakdown

If prerequisites are not met, inform the user and suggest running `/cm` first.

## CRITICAL: Scope Limitations

This skill ONLY updates:
- `.cm/PLAN.md` - Refine phases into detailed sub-phases

This skill does NOT:
- Generate tasks.json or roadmap.json (that comes later)
- Implement any code
- Run `cm run` or execute tasks

**Prerequisites:**
- `.cm/PLAN.md` must exist (run `/cm` first)
- tasks.json should NOT exist yet (this is a pre-generation step)

---

## Step 1: Load Current Plan

**Read the current PLAN.md:**

1. Read `.cm/PLAN.md` to understand existing phases
2. Parse the "## Phases" section

**Present summary to user:**

> **Current High-Level Plan:**
>
> **Phases from PLAN.md:**
> 1. Phase 1: [name] - [brief description]
> 2. Phase 2: [name] - [brief description]
> ...
>
> Would you like me to analyze and split these into more detailed phases?

Wait for user confirmation.

---

## Step 2: Analyze Phases

For each phase, evaluate:
- Current scope and complexity
- Number and size of tasks
- Dependencies between tasks
- Whether it can be meaningfully split

**Present analysis:**

> **Phase Analysis:**
>
> **[phase-id]: [phase-name]**
> - Current tasks: [N]
> - Complexity: [low/medium/high]
> - Recommendation: [Keep as-is / Split into N sub-phases]
> - Suggested split: [Brief description of how to split]
>
> [Repeat for each phase]
>
> Do you want me to proceed with splitting? You can also specify which phases to split.

Wait for user input.

---

## Step 3: Generate Detailed Phases

For each phase being split:

1. **Identify logical sub-divisions:**
   - By component/module
   - By feature area
   - By dependency order
   - By complexity layer (foundation → advanced)

2. **Create new phase definitions:**
   - New phase IDs: `phase-N.1`, `phase-N.2`, etc. (or new sequential numbers)
   - Clear, specific names
   - Detailed task breakdown
   - Proper dependencies

3. **Preserve existing work:**
   - Keep completed phases intact
   - Maintain task history
   - Update dependencies correctly

**Present proposed split:**

> **Proposed Phase Split:**
>
> **Original:** phase-1 "Core Implementation" (5 tasks)
>
> **Split into:**
> - phase-1.1 "Data Models" (2 tasks)
>   - task-1: Define structs
>   - task-2: Add serialization
>
> - phase-1.2 "Business Logic" (2 tasks)
>   - task-3: Core algorithms
>   - task-4: Error handling
>
> - phase-1.3 "Integration" (1 task)
>   - task-5: Wire components together
>
> Does this look correct? Reply "yes" to update files, or provide corrections.

Wait for user confirmation.

---

## Step 4: Update PLAN.md

After confirmation, update `.cm/PLAN.md`:

### 4.1 Replace Phases Section

Replace the original "## Phases" section with detailed sub-phases:

```markdown
## Phases

### Phase 1: Data Models
**Goal:** Define core data structures
**Tasks:**
- Define User struct with fields
- Define Project struct with relationships
- Add serde serialization

### Phase 2: Business Logic
**Goal:** Implement core algorithms
**Tasks:**
- Validation functions
- Error handling
- State transitions

### Phase 3: Integration
**Goal:** Wire components together
**Tasks:**
- Connect data layer to logic
- Add API endpoints
```

### 4.2 Preserve Other Sections

Keep all other PLAN.md sections intact:
- Overview, Goals, Success Criteria
- Architecture, Modules
- Technical Decisions, Out of Scope

### 4.3 No JSON Generation Yet

**Important:** Do NOT generate tasks.json or roadmap.json at this step.
Tell the user the next step is to continue with `/cm` or run JSON generation.

---

## Step 5: Completion Summary

> **Split completed successfully!**
>
> **Changes to PLAN.md:**
> - Original phases: [N]
> - New detailed phases: [M]
> - Total tasks outlined: [T]
>
> **New phase structure:**
> 1. Phase 1: [name]
> 2. Phase 2: [name]
> ...
>
> **Next steps:**
> 1. Review `.cm/PLAN.md` for the detailed structure
> 2. Continue with `/cm` to generate tasks.json and roadmap.json
> 3. Or manually trigger JSON generation when ready

---

## Phase Split Guidelines

### When to Split a Phase

Split when:
- Phase has more than 5-7 tasks
- Tasks span multiple components
- Tasks have complex interdependencies
- Phase scope is too broad to track easily

### How to Split

1. **By Component:** Group tasks that touch the same module/file
2. **By Layer:** Foundation tasks first, then features, then polish
3. **By Feature:** Separate distinct feature areas
4. **By Dependency:** Independent tasks in parallel phases

### Naming Conventions

- Use descriptive names: "User Authentication" not "Phase 2"
- Keep names concise: 3-5 words max
- Indicate scope: "API Endpoints" vs "Full Backend"

### ID Conventions

Option A - Sub-numbering:
- `phase-1` → `phase-1.1`, `phase-1.2`, `phase-1.3`

Option B - Sequential:
- Insert new phases with new numbers
- Renumber subsequent phases if needed

---

## Error Handling

### No PLAN.md found
> I couldn't find `.cm/PLAN.md`. Please run `/cm` first to create the initial plan.

### tasks.json already exists
> Warning: `tasks.json` already exists. `/split` is meant to be run before JSON generation.
> Options:
> 1. Delete tasks.json and roadmap.json, then run /split
> 2. Use /feat to add features to the existing plan instead

### No phases to split
> All phases in PLAN.md are already well-detailed. No further splitting recommended.

### PLAN.md has no Phases section
> PLAN.md doesn't have a "## Phases" section. Please ensure /cm completed properly.
```

### 2. Add to `src/command.rs`

```rust
pub const SPLIT_COMMAND: &str = include_str!("../assets/split.md");
```

Also add to the exports and COMMANDS array.

### 3. Update `src/cli/init.rs`

Add to COMMANDS array:
```rust
CommandFile {
    name: "split",
    content: SPLIT_COMMAND,
},
```

---

# Part 2: Extend /end for /cm Sessions

## Current /end Behavior
Currently `/end` only works after `/feat` or `/fix` sessions:
- Reads gathered feature/fix info from conversation
- Adds tasks to existing tasks.json
- Adds items to existing roadmap.json
- Runs validation

## New /end Behavior for /cm Sessions

When `/end` is run after a `/cm` (or `/cm` + `/split`) session:

### Step 1: Detect Session Type

Check what exists:
- If `tasks.json` exists → This is a `/feat` or `/fix` session (existing behavior)
- If only `PLAN.md` exists → This is a `/cm` session (new behavior)

### Step 2: Parse PLAN.md

Extract from PLAN.md:
- Project name and description
- Phases section (with tasks for each phase)
- Success criteria
- Architecture info

### Step 3: Generate tasks.json

Create tasks.json from PLAN.md phases:

```json
{
  "version": "1.0.0",
  "project": {
    "name": "[from PLAN.md]",
    "description": "[from PLAN.md]",
    "created_at": "[timestamp]"
  },
  "global_context": {
    "plan_summary": "[from PLAN.md overview]"
  },
  "phases": [
    {
      "id": "phase-1",
      "name": "[Phase name from PLAN.md]",
      "plan": "",
      "status": "pending",
      "tasks": [
        {
          "id": "phase-1.task-1",
          "name": "[Task from PLAN.md]",
          "type": "implement",
          "status": "pending",
          "depends_on": [],
          "context": {
            "files_to_read": [],
            "prior_review_issues": []
          },
          "plan_file": ".cm/plans/plan-1.md",
          "roadmap_item_id": "phase-1-item-1"
        }
      ]
    }
  ],
  "agent_history": [],
  "log_records": []
}
```

### Step 4: Generate roadmap.json

Create roadmap.json from PLAN.md phases:

```json
{
  "version": "1.0.0",
  "title": "[Project name]",
  "phases": [
    {
      "id": "phase-1",
      "number": "1",
      "name": "[Phase name]",
      "items": [
        {
          "id": "phase-1-item-1",
          "name": "[Task name]",
          "completed": false,
          "linked_task_ids": ["phase-1.task-1"]
        }
      ]
    }
  ]
}
```

### Step 5: Create Plan Files

For each phase, create `.cm/plans/plan-N.md` with task details.

### Step 6: Regenerate & Validate

```bash
cm --regenerate
cm --sanity-check
```

### Step 7: Summary

> **Project initialized successfully!**
>
> **Generated files:**
> - `.cm/tasks.json` - [N] phases, [M] tasks
> - `.cm/roadmap.json` - [N] phases
> - `.cm/plans/plan-*.md` - Phase plan files
>
> **Next steps:**
> 1. Review generated files
> 2. Run `cm` to start implementation

## Files to Modify

### `assets/end.md`

Add session detection at the start:

```markdown
## Step 1: Detect Session Type

First, determine what type of session you're finalizing:

1. Check if `.cm/tasks.json` exists:
   - **If YES** → This is a `/feat` or `/fix` session (continue with existing flow)
   - **If NO** → This is a `/cm` session (use new JSON generation flow)

2. For `/cm` sessions, check if `.cm/PLAN.md` exists:
   - **If NO** → Error: Run `/cm` first
   - **If YES** → Proceed with JSON generation
```

Add new section for /cm finalization after existing /feat and /fix handling.

---

# Files Summary

## New Files
- `assets/split.md` - /split command wizard

## Modified Files
- `assets/end.md` - Add /cm session support
- `src/command.rs` - Add `SPLIT_COMMAND` constant
- `src/cli/init.rs` - Deploy split.md during `cm init`

---

## Testing

### /split Testing
1. Run `cm init --force` to deploy the new command
2. Run `/cm` to create PLAN.md (stop before JSON generation)
3. Run `/split` to refine phases
4. Verify PLAN.md is updated with detailed phases

### /end for /cm Testing
1. After `/cm` (and optionally `/split`), run `/end`
2. Verify tasks.json is generated correctly
3. Verify roadmap.json is generated correctly
4. Verify `cm --sanity-check` passes
