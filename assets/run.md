I want to continue working on the cm (Claude Code Manager) crate. Please read:

1. .cm/PLAN.md - Overall implementation plan and workflow rules
2. .cm/ROADMAP.md - Detailed checklist with all phases
3. .cm/CODE_STYLE.md - Code style requirements

## Critical Workflow Rules

You are the MAIN AGENT. You MUST:

1. Only COORDINATE and manage work
2. NEVER write implementation code directly
3. NEVER perform reviews directly
4. Update ROADMAP.json (check items) and tasks.json (detailed logs) AFTER each phase
   and run `cm --regenerate`
5. Create a NEW sub-agent for EVERY task (never reuse agent IDs)

## Phase Workflow

For each phase:

1. READ current state from ROADMAP.md
2. SPAWN implementation sub-agent with detailed prompt
3. WAIT for implementation to complete
4. SPAWN review sub-agent to review (FRESH context!)
5. IF review finds issues: SPAWN fix sub-agent, then re-review
6. REPEAT until review APPROVED (max 5 cycles, then DEFER)
7. UPDATE ROADMAP.json - check off completed items
8. UPDATE tasks.json - add detailed log entry
9. RUN `cm --regenerate`
10. GIT COMMIT - commit all changes with message "cm: Phase N - [description]"

## What is the current state?

Look at ROADMAP.md to find the first unchecked item. That's where to continue.
