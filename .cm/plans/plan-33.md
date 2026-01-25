# Plan: Add PLAN Agent Before IMPLEM

## Summary
Before spawning an IMPLEM agent, first spawn a PLAN agent that evaluates the initial plan and optionally generates a more detailed plan. If the PLAN agent returns `plan: null`, use the original plan; otherwise use the detailed plan.

## Workflow

```
execute_phase(phase_id)
    │
    ├─ Build PHASE_PLAN prompt (with initial plan)
    ├─ Spawn PLAN agent
    ├─ Parse response: { "plan": "detailed..." } or { "plan": null }
    │
    ├─ If detailed plan provided:
    │      └─ Use detailed plan for IMPLEM
    ├─ If plan: null:
    │      └─ Use original plan for IMPLEM
    │
    ├─ Spawn IMPLEM agent (existing flow)
    └─ ... (rest continues as before)
```

## Files to Modify

1. `src/agent/prompt.rs` - Add `build_phase_plan_prompt()`
2. `src/agent/response.rs` - Add `PlanAgentResponse` struct
3. `src/manager/mod.rs` - Add plan agent step in `execute_phase()`
4. `assets/templates/PLANNER.md` - New template for plan agent
5. `src/command.rs` - Add `PLANNER_TEMPLATE` constant

## Implementation Steps

### 1. Create PLANNER template (`assets/templates/PLANNER.md`)

```markdown
# Planner Agent Instructions

You are a **Planner Agent**. Your role is to evaluate an implementation plan and decide whether to create a more detailed version.

## Your Task

1. Read the provided implementation plan
2. Evaluate if it needs more detail for successful implementation
3. If you can add valuable detail: return a comprehensive plan
4. If the plan is already detailed enough: return null

## When to Create a Detailed Plan

Create a detailed plan when:
- The plan lacks specific file paths or line numbers
- Implementation steps are vague or ambiguous
- Error handling requirements are unclear
- The order of operations isn't specified
- Edge cases aren't addressed

Return null when:
- The plan is already step-by-step with clear instructions
- File paths and changes are explicitly specified
- The task is simple enough that more detail would be redundant

## Required Output Format

You MUST end your response with a JSON code block:

If you created a detailed plan:
```json
{
  "plan": "# Detailed Implementation Plan\n\n## Step 1: ...\n\n## Step 2: ..."
}
```

If the original plan is sufficient:
```json
{
  "plan": null
}
```
```

### 2. Add `PLANNER_TEMPLATE` to `src/command.rs`

```rust
pub const PLANNER_TEMPLATE: &str = include_str!("../assets/templates/PLANNER.md");
```

### 3. Add `PlanAgentResponse` to `src/agent/response.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct PlanAgentResponse {
    pub plan: Option<String>,
}
```

Add parsing method:
```rust
pub fn parse_plan_response(output: &str) -> Result<PlanAgentResponse, String> {
    // Extract JSON block and parse
    // Return PlanAgentResponse { plan: Some(detailed) } or { plan: None }
}
```

### 4. Add `build_phase_plan_prompt()` to `src/agent/prompt.rs`

```rust
pub fn build_phase_plan_prompt(
    &self,
    phase: &Phase,
    initial_plan: &str,
) -> Result<String, StateError> {
    let mut prompt = String::new();

    // Load PLANNER template
    prompt.push_str(&self.load_template("PLANNER.md")?);
    prompt.push_str("\n\n---\n\n");

    // Add phase context
    prompt.push_str(&format!("# Phase: {}\n\n", phase.name));

    // Add initial plan
    prompt.push_str("## Initial Plan\n\n");
    prompt.push_str(initial_plan);
    prompt.push_str("\n\n");

    // Add task list for context
    prompt.push_str("## Tasks in This Phase\n\n");
    for task in &phase.tasks {
        prompt.push_str(&format!("- **{}**: {}\n", task.id, task.name));
    }

    Ok(prompt)
}
```

### 5. Modify `execute_phase()` in `src/manager/mod.rs`

Add plan agent step before IMPLEM:

```rust
// === NEW: Run PLAN agent first ===
let initial_plan = self.load_phase_plan(phase_id)?;  // Load from plan_file

let plan_prompt = self.prompt_builder.build_phase_plan_prompt(&phase, &initial_plan)?;
info!("Spawning PLAN agent for {}", phase_id);

// Log prompt (same as other agents)
if let Some(logger) = &self.phase_logger {
    logger.log_prompt("PHASE_PLAN", &plan_prompt)?;
}

// Record agent spawn in tasks.json log_records
let plan_agent_id = uuid::Uuid::new_v4().to_string();
self.state.log_records.push(LogRecord {
    id: uuid::Uuid::new_v4().to_string(),
    timestamp: Utc::now(),
    action: "agent_spawn".to_string(),
    task_id: Some(phase_id.to_string()),
    data: LogData::AgentSpawn {
        agent_type: "plan".to_string(),
        prompt_preview: plan_prompt.chars().take(500).collect(),
    },
});

let plan_handle = self.agent_spawner.spawn(&plan_prompt, phase_id, "PHASE_PLAN")?;
let plan_output = plan_handle.wait()?;

// Log response (same as other agents)
if let Some(logger) = &self.phase_logger {
    logger.log_response("PHASE_PLAN", &plan_output.stdout)?;
}

let plan_response = ResponseParser::parse_plan_response(&plan_output.stdout)
    .map_err(|e| ManagerError::AgentError(AgentError::ParseError(e)))?;

// Use detailed plan if provided, else original
let final_plan = match plan_response.plan {
    Some(detailed) => {
        info!("PLAN agent provided detailed plan");
        detailed
    }
    None => {
        info!("PLAN agent: original plan sufficient");
        initial_plan
    }
};

// === Continue with existing IMPLEM flow, but use final_plan ===
let implem_prompt = self.prompt_builder.build_phase_implem_prompt_with_plan(
    &phase,
    &pending_tasks,
    &final_plan,  // Use the (possibly detailed) plan
)?;
```

### 6. Update `build_phase_implem_prompt()` signature

Add a variant that accepts an explicit plan string instead of loading from file:

```rust
pub fn build_phase_implem_prompt_with_plan(
    &self,
    phase: &Phase,
    tasks: &[&Task],
    plan: &str,
) -> Result<String, StateError>
```

## Testing

1. `cargo build` and `cargo clippy` must pass
2. `cargo test` must pass
3. Manual test: run `cm --step` on a phase and verify:
   - PLAN agent runs first
   - If it returns a detailed plan, IMPLEM uses it
   - If it returns null, IMPLEM uses original plan
