//! Response parsing for agent output.
//!
//! This module provides functionality to parse JSON responses from Claude agents,
//! extracting structured information about files created/modified and commands run.

use serde::{Deserialize, Serialize};

use crate::state::{AgentResponse, AgentStatus};

use super::AgentError;

/// Response from a phase-level agent that implements multiple tasks.
///
/// This is the structured response expected from `build_phase_implem_prompt()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAgentResponse {
    /// Status reported by the agent (success/failed).
    #[serde(default = "default_agent_status")]
    pub status: AgentStatus,
    /// Summary of the entire phase implementation.
    #[serde(default)]
    pub summary: String,
    /// Information about each completed task.
    #[serde(default)]
    pub tasks_completed: Vec<TaskCompletionInfo>,
    /// Files created during phase implementation.
    #[serde(default)]
    pub files_created: Vec<String>,
    /// Files modified during phase implementation.
    #[serde(default)]
    pub files_modified: Vec<String>,
    /// Error message if status is failed.
    #[serde(default)]
    pub error: Option<String>,
}

fn default_agent_status() -> AgentStatus {
    AgentStatus::Success
}

/// Information about a single completed task within a phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionInfo {
    /// The task ID (e.g., "phase-1.task-2").
    pub task_id: String,
    /// Summary of what was done for this task.
    #[serde(default)]
    pub summary: String,
}

/// Response from a review agent.
///
/// This is the structured response expected from review prompts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAgentResponse {
    /// Status reported by the agent (success/failed).
    #[serde(default = "default_agent_status")]
    pub status: AgentStatus,
    /// Review verdict: "approved" or "needs_fixes".
    #[serde(default)]
    pub verdict: Option<String>,
    /// Summary of the review.
    #[serde(default)]
    pub summary: String,
    /// List of issues found during review.
    #[serde(default)]
    pub issues: Vec<ReviewIssueResponse>,
    /// Error message if status is failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// A single issue found during code review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssueResponse {
    /// Unique identifier for this issue.
    #[serde(default)]
    pub id: String,
    /// Severity level: "critical", "high", "medium", or "low".
    #[serde(default)]
    pub severity: String,
    /// File and line location (e.g., "src/main.rs:42").
    #[serde(default)]
    pub location: String,
    /// Description of the problem.
    #[serde(default)]
    pub problem: String,
    /// Suggested fix for the issue.
    #[serde(default)]
    pub suggested_fix: String,
}

/// Response from a plan agent.
///
/// The plan agent evaluates an initial plan and optionally returns a more detailed version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAgentResponse {
    /// The detailed plan if created, or null if the original plan is sufficient.
    pub plan: Option<String>,
}

/// Response from the post-run review agent.
///
/// The agent prints a markdown report and then a trailing JSON line with this structure.
#[derive(Debug, Clone, Deserialize)]
pub struct RunReviewAgentResponse {
    /// Whether the review agent found any issues.
    pub has_issues: bool,
    /// One-line summary of the review findings.
    #[serde(default)]
    pub summary: String,
}

/// Parses raw JSON output from Claude agents.
///
/// The parser handles both well-formed JSON and gracefully handles
/// malformed responses by extracting what it can.
pub struct ResponseParser;

/// Raw JSON structure from claude CLI output (legacy format).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClaudeJsonOutput {
    #[serde(default)]
    result: String,
    #[serde(default)]
    cost_usd: Option<f64>,
    #[serde(default)]
    session_id: Option<String>,
}


/// Structure for extracting structured info from the agent's JSON response.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct ParsedResponse {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    files_created: Vec<String>,
    #[serde(default)]
    files_modified: Vec<String>,
    #[serde(default)]
    commands_run: Vec<String>,
}

impl ResponseParser {
    /// Extract session_id from raw JSON output, even if full parsing fails.
    ///
    /// This is useful for retry scenarios where we need the session ID
    /// to continue the conversation but the full response parsing failed.
    /// Uses string search fallback when serde parsing fails.
    pub fn extract_session_id(raw_json: &str) -> Option<String> {
        // Try serde first
        #[derive(Deserialize)]
        struct MinimalOutput {
            session_id: Option<String>,
        }
        if let Ok(output) = serde_json::from_str::<MinimalOutput>(raw_json) {
            if let Some(id) = output.session_id {
                return Some(id);
            }
        }

        // Fallback: simple string search for "session_id":"<value>"
        let marker = "\"session_id\":\"";
        if let Some(start) = raw_json.find(marker) {
            let value_start = start + marker.len();
            if let Some(end) = raw_json[value_start..].find('"') {
                return Some(raw_json[value_start..value_start + end].to_string());
            }
        }
        None
    }

    /// Parse raw JSON output from claude CLI into an AgentResponse.
    ///
    /// The parser handles two formats:
    /// 1. Legacy format: `{"result": "...", "session_id": "..."}`
    /// 2. Streaming format: `[{"type":"system",...}, {"type":"result","result":"..."}]`
    ///
    /// # Errors
    ///
    /// Returns `AgentError::ParseError` if the JSON cannot be parsed at all.
    pub fn parse(raw_json: &str) -> Result<AgentResponse, AgentError> {
        let trimmed = raw_json.trim();

        // Check for empty output first
        if trimmed.is_empty() {
            return Err(AgentError::ParseError(
                "claude CLI returned empty output (no stdout)".to_string(),
            ));
        }

        // Check if it's a JSON array (streaming format)
        if trimmed.starts_with('[') {
            return Self::parse_streaming_format(trimmed);
        }

        // Try legacy format
        let output: ClaudeJsonOutput = serde_json::from_str(raw_json).map_err(|e| {
            AgentError::ParseError(format!("failed to parse claude CLI output: {e}"))
        })?;

        let result_text = output.result;
        let parsed = Self::extract_embedded_json(&result_text);

        Ok(Self::build_response(parsed, &result_text))
    }

    /// Parse streaming JSON array format from claude CLI.
    ///
    /// Uses serde_json::Value for flexibility since events have many varying fields.
    fn parse_streaming_format(raw_json: &str) -> Result<AgentResponse, AgentError> {
        let events: Vec<serde_json::Value> = serde_json::from_str(raw_json).map_err(|e| {
            AgentError::ParseError(format!("failed to parse streaming output: {e}"))
        })?;

        let mut result_text = String::new();

        for event in &events {
            let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

            // Check for "result" type event
            if event_type == "result" {
                if let Some(result) = event.get("result").and_then(|v| v.as_str()) {
                    result_text = result.to_string();
                    break;
                }
            }
            // Also check for assistant messages with content
            if event_type == "assistant" {
                if let Some(content) = event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    result_text.push_str(content);
                }
            }
        }

        let parsed = Self::extract_embedded_json(&result_text);

        Ok(Self::build_response(parsed, &result_text))
    }

    /// Build an `AgentResponse` from a `ParsedResponse` and the full result text.
    ///
    /// Maps the `status`/`summary`/`error` fields from the parsed JSON into
    /// the `AgentStatus` and `message` fields on `AgentResponse`.
    fn build_response(parsed: ParsedResponse, result_text: &str) -> AgentResponse {
        let status = match parsed.status.as_deref() {
            Some("failed") => AgentStatus::Failed,
            _ => AgentStatus::Success,
        };

        // Message: use summary (success) or error (failed), fall back to result_text
        let message = if status == AgentStatus::Failed {
            parsed.error.unwrap_or_else(|| result_text.to_string())
        } else {
            parsed.summary.unwrap_or_else(|| result_text.to_string())
        };

        AgentResponse {
            status,
            files_created: parsed.files_created,
            files_modified: parsed.files_modified,
            commands_run: parsed.commands_run,
            message,
        }
    }

    /// Attempt to extract embedded JSON from the response text.
    ///
    /// Claude responses often contain JSON embedded in markdown code blocks
    /// or inline in the text. This function attempts to find and parse that JSON.
    fn extract_embedded_json(text: &str) -> ParsedResponse {
        // Try to find JSON in code blocks first
        if let Some(json_str) = Self::extract_json_from_code_block(text) {
            if let Ok(parsed) = serde_json::from_str::<ParsedResponse>(&json_str) {
                return parsed;
            }
        }

        // Try to find raw JSON object in the text
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    let potential_json = &text[start..=end];
                    if let Ok(parsed) = serde_json::from_str::<ParsedResponse>(potential_json) {
                        return parsed;
                    }
                }
            }
        }

        // If no JSON found, return empty defaults
        ParsedResponse::default()
    }

    /// Parse raw JSON output from claude CLI into a PhaseAgentResponse.
    ///
    /// Similar to `parse()` but returns a `PhaseAgentResponse` which includes
    /// multi-task completion information.
    ///
    /// # Errors
    ///
    /// Returns `AgentError::ParseError` if the JSON cannot be parsed at all.
    pub fn parse_phase_response(raw_json: &str) -> Result<PhaseAgentResponse, AgentError> {
        let trimmed = raw_json.trim();

        // Get the result text from the raw JSON
        let result_text = if trimmed.starts_with('[') {
            Self::extract_result_from_streaming(trimmed)?
        } else {
            let output: ClaudeJsonOutput = serde_json::from_str(raw_json).map_err(|e| {
                AgentError::ParseError(format!("failed to parse claude CLI output: {e}"))
            })?;
            output.result
        };

        // Try to extract embedded JSON from the result text
        let parsed = Self::extract_phase_response_json(&result_text);

        Ok(parsed.unwrap_or_else(|| PhaseAgentResponse {
            status: AgentStatus::Success,
            summary: result_text,
            tasks_completed: vec![],
            files_created: vec![],
            files_modified: vec![],
            error: None,
        }))
    }

    /// Extract result text from streaming format.
    fn extract_result_from_streaming(raw_json: &str) -> Result<String, AgentError> {
        let events: Vec<serde_json::Value> = serde_json::from_str(raw_json).map_err(|e| {
            AgentError::ParseError(format!("failed to parse streaming output: {e}"))
        })?;

        let mut result_text = String::new();

        for event in &events {
            let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

            if event_type == "result" {
                if let Some(result) = event.get("result").and_then(|v| v.as_str()) {
                    result_text = result.to_string();
                    break;
                }
            }
            if event_type == "assistant" {
                if let Some(content) = event
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    result_text.push_str(content);
                }
            }
        }

        Ok(result_text)
    }

    /// Attempt to extract a PhaseAgentResponse from embedded JSON in the text.
    fn extract_phase_response_json(text: &str) -> Option<PhaseAgentResponse> {
        // Try to find JSON in code blocks first
        if let Some(json_str) = Self::extract_json_from_code_block(text) {
            if let Ok(parsed) = serde_json::from_str::<PhaseAgentResponse>(&json_str) {
                return Some(parsed);
            }
        }

        // Try to find raw JSON object in the text
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    let potential_json = &text[start..=end];
                    if let Ok(parsed) = serde_json::from_str::<PhaseAgentResponse>(potential_json) {
                        return Some(parsed);
                    }
                }
            }
        }

        None
    }

    /// Parse raw JSON output from claude CLI into a ReviewAgentResponse.
    ///
    /// Similar to `parse()` but returns a `ReviewAgentResponse` which includes
    /// verdict and issues information for review agents.
    ///
    /// # Errors
    ///
    /// Returns `AgentError::ParseError` if the JSON cannot be parsed at all.
    pub fn parse_review_response(raw_json: &str) -> Result<ReviewAgentResponse, AgentError> {
        let trimmed = raw_json.trim();

        // Get the result text from the raw JSON
        let result_text = if trimmed.starts_with('[') {
            Self::extract_result_from_streaming(trimmed)?
        } else {
            let output: ClaudeJsonOutput = serde_json::from_str(raw_json).map_err(|e| {
                AgentError::ParseError(format!("failed to parse claude CLI output: {e}"))
            })?;
            output.result
        };

        // Try to extract embedded JSON from the result text
        let parsed = Self::extract_review_response_json(&result_text);

        Ok(parsed.unwrap_or_else(|| ReviewAgentResponse {
            status: AgentStatus::Success,
            verdict: None,
            summary: result_text,
            issues: vec![],
            error: None,
        }))
    }

    /// Attempt to extract a ReviewAgentResponse from embedded JSON in the text.
    fn extract_review_response_json(text: &str) -> Option<ReviewAgentResponse> {
        // Try to find JSON in code blocks first
        if let Some(json_str) = Self::extract_json_from_code_block(text) {
            if let Ok(parsed) = serde_json::from_str::<ReviewAgentResponse>(&json_str) {
                return Some(parsed);
            }
        }

        // Try to find raw JSON object in the text
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    let potential_json = &text[start..=end];
                    if let Ok(parsed) = serde_json::from_str::<ReviewAgentResponse>(potential_json) {
                        return Some(parsed);
                    }
                }
            }
        }

        None
    }

    /// Parse raw JSON output from claude CLI into a PlanAgentResponse.
    ///
    /// Similar to `parse_review_response()` but returns a `PlanAgentResponse`.
    ///
    /// # Errors
    ///
    /// Returns `AgentError::ParseError` if the JSON cannot be parsed at all.
    pub fn parse_plan_response(raw_json: &str) -> Result<PlanAgentResponse, AgentError> {
        let trimmed = raw_json.trim();

        // Get the result text from the raw JSON
        let result_text = if trimmed.starts_with('[') {
            Self::extract_result_from_streaming(trimmed)?
        } else {
            let output: ClaudeJsonOutput = serde_json::from_str(raw_json).map_err(|e| {
                AgentError::ParseError(format!("failed to parse claude CLI output: {e}"))
            })?;
            output.result
        };

        // Try to extract embedded JSON from the result text
        let parsed = Self::extract_plan_response_json(&result_text);

        Ok(parsed.unwrap_or(PlanAgentResponse { plan: None }))
    }

    /// Attempt to extract a PlanAgentResponse from embedded JSON in the text.
    fn extract_plan_response_json(text: &str) -> Option<PlanAgentResponse> {
        // Try to find JSON in code blocks first
        if let Some(json_str) = Self::extract_json_from_code_block(text) {
            if let Ok(parsed) = serde_json::from_str::<PlanAgentResponse>(&json_str) {
                return Some(parsed);
            }
        }

        // Try to find raw JSON object in the text
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    let potential_json = &text[start..=end];
                    if let Ok(parsed) = serde_json::from_str::<PlanAgentResponse>(potential_json) {
                        return Some(parsed);
                    }
                }
            }
        }

        None
    }

    /// Parse the output from a RunReview agent.
    ///
    /// The agent produces a markdown report followed by a trailing JSON block.
    /// Extracts the last `{...}` object in the output and parses it as
    /// `RunReviewAgentResponse`. On any parse failure, returns `has_issues: true`
    /// (fail-safe: unknown state is treated as issues found).
    ///
    /// Returns `(report_text, response)` where `report_text` is the markdown
    /// portion (everything before the trailing JSON block).
    pub fn parse_run_review_response(output: &str) -> (String, RunReviewAgentResponse) {
        // Find the last '{' in the output — that's where the trailing JSON starts
        if let Some(json_start) = output.rfind('{') {
            let json_candidate = &output[json_start..];
            if let Ok(resp) = serde_json::from_str::<RunReviewAgentResponse>(json_candidate) {
                let report_text = output[..json_start].trim_end().to_string();
                return (report_text, resp);
            }
        }

        // Parse failed — conservative default: treat as issues found
        (
            output.to_string(),
            RunReviewAgentResponse {
                has_issues: true,
                summary: "parse error".to_string(),
            },
        )
    }

    /// Extract JSON from markdown code blocks.
    ///
    /// Looks for ```json or ``` code blocks and extracts the content.
    fn extract_json_from_code_block(text: &str) -> Option<String> {
        // Try to find ```json block first
        let json_block_start = text.find("```json");
        if let Some(start) = json_block_start {
            let content_start = start + 7; // Skip "```json"
            if let Some(end) = text[content_start..].find("```") {
                let json_str = text[content_start..content_start + end].trim();
                return Some(json_str.to_string());
            }
        }

        // Try plain code block
        let code_block_start = text.find("```\n");
        if let Some(start) = code_block_start {
            let content_start = start + 4; // Skip "```\n"
            if let Some(end) = text[content_start..].find("```") {
                let content = text[content_start..content_start + end].trim();
                // Only return if it looks like JSON
                if content.starts_with('{') {
                    return Some(content.to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_claude_output() {
        let raw = r#"{
            "result": "I created the file.\n\n```json\n{\"files_created\": [\"src/foo.rs\"], \"files_modified\": [], \"commands_run\": [\"cargo build\"]}\n```",
            "cost_usd": 0.01,
            "session_id": "abc123"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.files_created, vec!["src/foo.rs"]);
        assert!(response.files_modified.is_empty());
        assert_eq!(response.commands_run, vec!["cargo build"]);
    }

    #[test]
    fn test_parse_with_inline_json() {
        let raw = r#"{
            "result": "Done! Here's the result: {\"files_created\": [\"src/bar.rs\"], \"files_modified\": [\"src/lib.rs\"], \"commands_run\": []}"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.files_created, vec!["src/bar.rs"]);
        assert_eq!(response.files_modified, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_parse_without_embedded_json() {
        let raw = r#"{
            "result": "I made some changes to the code but didn't provide structured output."
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        // Should have empty arrays but preserve the raw response
        assert!(response.files_created.is_empty());
        assert!(response.files_modified.is_empty());
        assert!(response.commands_run.is_empty());
        assert!(response.message.contains("made some changes"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let raw = "not valid json at all";

        let result = ResponseParser::parse(raw);

        assert!(matches!(result, Err(AgentError::ParseError(_))));
    }

    #[test]
    fn test_parse_malformed_embedded_json() {
        let raw = r#"{
            "result": "Here's some broken JSON: {\"files_created\": [incomplete"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        // Should return empty arrays when embedded JSON is malformed
        assert!(response.files_created.is_empty());
        assert!(response.message.contains("broken JSON"));
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let text = "Some text\n```json\n{\"key\": \"value\"}\n```\nMore text";
        let result = ResponseParser::extract_json_from_code_block(text);

        assert!(result.is_some());
        assert!(result.unwrap().contains("\"key\""));
    }

    #[test]
    fn test_extract_json_from_plain_code_block() {
        let text = "Some text\n```\n{\"key\": \"value\"}\n```\nMore text";
        let result = ResponseParser::extract_json_from_code_block(text);

        assert!(result.is_some());
    }

    #[test]
    fn test_extract_no_json_code_block() {
        let text = "Just plain text without any code blocks";
        let result = ResponseParser::extract_json_from_code_block(text);

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_preserves_message() {
        let raw = r#"{
            "result": "This is the full response text with all details."
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.message, "This is the full response text with all details.");
    }

    #[test]
    fn test_parse_with_empty_result() {
        let raw = r#"{
            "result": ""
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert!(response.message.is_empty());
        assert!(response.files_created.is_empty());
    }

    #[test]
    fn test_parse_handles_missing_fields() {
        let raw = r#"{
            "result": "```json\n{\"files_created\": [\"test.rs\"]}\n```"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.files_created, vec!["test.rs"]);
        // Other fields should default to empty
        assert!(response.files_modified.is_empty());
        assert!(response.commands_run.is_empty());
    }

    #[test]
    fn test_parse_streaming_format() {
        let raw = r#"[
            {"type":"system","subtype":"init","session_id":"abc123"},
            {"type":"result","result":"I created the file.\n\n```json\n{\"files_created\": [\"src/foo.rs\"]}\n```","session_id":"abc123"}
        ]"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.files_created, vec!["src/foo.rs"]);
        assert!(response.message.contains("I created the file"));
    }

    #[test]
    fn test_parse_streaming_format_with_assistant_message() {
        let raw = r#"[
            {"type":"system","subtype":"init","session_id":"abc123"},
            {"type":"assistant","message":{"content":"Done! Created src/bar.rs"}}
        ]"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert!(response.message.contains("Done!"));
    }

    #[test]
    fn test_parse_streaming_format_with_many_fields() {
        // Realistic streaming output with many extra fields
        let raw = r#"[
            {"type":"system","subtype":"init","cwd":"/home/user/cm","session_id":"73363420-7203-43c5-92f7-7cf0ddbcdfa2","tools":["Task","Bash","Glob","Grep","Read","Edit","Write"],"mcp_servers":[],"model":"claude-sonnet-4-5-20250929","permissionMode":"default","slash_commands":["feat","cm","fix"]},
            {"type":"assistant","message":{"id":"msg_123","type":"message","role":"assistant","content":[{"type":"text","text":"I'll create the file now."}],"model":"claude-sonnet-4-5-20250929","stop_reason":"end_turn","stop_sequence":null},"session_id":"73363420-7203-43c5-92f7-7cf0ddbcdfa2"},
            {"type":"result","subtype":"success","cost_usd":0.05,"is_error":false,"duration_ms":5000,"duration_api_ms":4500,"num_turns":1,"result":"I created the validation module.\n\n```json\n{\"files_created\": [\"src/state/validate.rs\"], \"files_modified\": [\"src/state/mod.rs\"]}\n```","session_id":"73363420-7203-43c5-92f7-7cf0ddbcdfa2"}
        ]"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.files_created, vec!["src/state/validate.rs"]);
        assert_eq!(response.files_modified, vec!["src/state/mod.rs"]);
        assert!(response.message.contains("validation module"));
    }

    #[test]
    fn test_extract_session_id_from_streaming() {
        let raw = r#"[{"type":"system","session_id":"abc-123-def"},{"type":"result","result":"done"}]"#;
        let session_id = ResponseParser::extract_session_id(raw);
        assert_eq!(session_id, Some("abc-123-def".to_string()));
    }

    #[test]
    fn test_parse_status_success_with_summary() {
        let raw = r#"{
            "result": "I did the work.\n\n```json\n{\"status\": \"success\", \"summary\": \"Created validation module\", \"files_created\": [\"src/validate.rs\"], \"files_modified\": [\"src/lib.rs\"]}\n```"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.status, AgentStatus::Success);
        assert_eq!(response.message, "Created validation module");
        assert_eq!(response.files_created, vec!["src/validate.rs"]);
        assert_eq!(response.files_modified, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_parse_status_failed_with_error() {
        let raw = r#"{
            "result": "I tried but couldn't do it.\n\n```json\n{\"status\": \"failed\", \"error\": \"Missing dependency: serde is not in Cargo.toml\"}\n```"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.status, AgentStatus::Failed);
        assert_eq!(response.message, "Missing dependency: serde is not in Cargo.toml");
        assert!(response.files_created.is_empty());
    }

    #[test]
    fn test_parse_no_status_defaults_to_success() {
        let raw = r#"{
            "result": "Did some work.\n\n```json\n{\"files_created\": [\"test.rs\"]}\n```"
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.status, AgentStatus::Success);
        assert_eq!(response.files_created, vec!["test.rs"]);
    }

    #[test]
    fn test_parse_review_response_approved() {
        let raw = r#"{
            "result": "Review complete.\n\n```json\n{\"status\": \"success\", \"verdict\": \"approved\", \"summary\": \"Code looks good\", \"issues\": []}\n```"
        }"#;

        let response = ResponseParser::parse_review_response(raw).unwrap();

        assert_eq!(response.status, AgentStatus::Success);
        assert_eq!(response.verdict, Some("approved".to_string()));
        assert_eq!(response.summary, "Code looks good");
        assert!(response.issues.is_empty());
    }

    #[test]
    fn test_parse_review_response_needs_fixes_with_issues() {
        let raw = r#"{
            "result": "Found issues.\n\n```json\n{\"status\": \"success\", \"verdict\": \"needs_fixes\", \"summary\": \"Found 2 issues\", \"issues\": [{\"id\": \"issue-1\", \"severity\": \"high\", \"location\": \"src/main.rs:42\", \"problem\": \"Missing error handling\", \"suggested_fix\": \"Add ? operator\"}]}\n```"
        }"#;

        let response = ResponseParser::parse_review_response(raw).unwrap();

        assert_eq!(response.status, AgentStatus::Success);
        assert_eq!(response.verdict, Some("needs_fixes".to_string()));
        assert_eq!(response.issues.len(), 1);
        assert_eq!(response.issues[0].id, "issue-1");
        assert_eq!(response.issues[0].severity, "high");
        assert_eq!(response.issues[0].location, "src/main.rs:42");
        assert_eq!(response.issues[0].problem, "Missing error handling");
    }

    #[test]
    fn test_parse_review_response_needs_fixes_empty_issues() {
        // This is the bug case - needs_fixes with 0 issues
        let raw = r#"{
            "result": "Review.\n\n```json\n{\"status\": \"success\", \"verdict\": \"needs_fixes\", \"summary\": \"Some feedback\", \"issues\": []}\n```"
        }"#;

        let response = ResponseParser::parse_review_response(raw).unwrap();

        assert_eq!(response.verdict, Some("needs_fixes".to_string()));
        assert!(response.issues.is_empty());
        // The manager should treat this as approved since there are no issues
    }

    #[test]
    fn test_parse_review_response_no_verdict() {
        let raw = r#"{
            "result": "Some review text without proper JSON"
        }"#;

        let response = ResponseParser::parse_review_response(raw).unwrap();

        assert_eq!(response.verdict, None);
        assert!(response.issues.is_empty());
    }

    #[test]
    fn test_parse_run_review_response_extracts_trailing_json() {
        let output = "## Report\n\nSome markdown text.\n\n{\"has_issues\": true, \"summary\": \"build failed\"}";
        let (report, result) = ResponseParser::parse_run_review_response(output);
        assert!(result.has_issues);
        assert_eq!(result.summary, "build failed");
        assert!(report.contains("Some markdown text."));
        assert!(!report.contains("has_issues"));
    }

    #[test]
    fn test_parse_run_review_response_no_issues() {
        let output = "All looks good.\n{\"has_issues\": false, \"summary\": \"clean run\"}";
        let (_report, result) = ResponseParser::parse_run_review_response(output);
        assert!(!result.has_issues);
        assert_eq!(result.summary, "clean run");
    }

    #[test]
    fn test_parse_run_review_response_parse_failure_defaults_to_issues() {
        let (_report, result) = ResponseParser::parse_run_review_response("no json here at all");
        assert!(result.has_issues); // conservative default
    }

    #[test]
    fn test_parse_run_review_response_malformed_json_defaults_to_issues() {
        let (_report, result) = ResponseParser::parse_run_review_response("some text {not valid json}");
        assert!(result.has_issues);
    }
}
