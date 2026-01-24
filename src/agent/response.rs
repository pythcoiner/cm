//! Response parsing for agent output.
//!
//! This module provides functionality to parse JSON responses from Claude agents,
//! extracting structured information about files created/modified and commands run.

use serde::Deserialize;

use crate::state::AgentResponse;

use super::AgentError;

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

/// Streaming event from claude CLI JSON array output.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamingEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    subtype: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    message: Option<StreamingMessage>,
}

/// Message content in streaming events.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamingMessage {
    #[serde(default)]
    content: Option<String>,
}

/// Structure for extracting file/command info from the response text.
/// Some fields like `summary` are not used directly but are needed for correct JSON parsing.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct ParsedResponse {
    #[serde(default)]
    files_created: Vec<String>,
    #[serde(default)]
    files_modified: Vec<String>,
    #[serde(default)]
    commands_run: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
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

        // Check if it's a JSON array (streaming format)
        if trimmed.starts_with('[') {
            return Self::parse_streaming_format(trimmed);
        }

        // Try legacy format
        let output: ClaudeJsonOutput = serde_json::from_str(raw_json).map_err(|e| {
            AgentError::ParseError(format!("failed to parse claude CLI output: {}", e))
        })?;

        let raw_response = output.result;
        let parsed = Self::extract_embedded_json(&raw_response);

        Ok(AgentResponse {
            files_created: parsed.files_created,
            files_modified: parsed.files_modified,
            commands_run: parsed.commands_run,
            raw_response,
        })
    }

    /// Parse streaming JSON array format from claude CLI.
    fn parse_streaming_format(raw_json: &str) -> Result<AgentResponse, AgentError> {
        let events: Vec<StreamingEvent> = serde_json::from_str(raw_json).map_err(|e| {
            AgentError::ParseError(format!("failed to parse streaming output: {}", e))
        })?;

        // Look for result in events
        let mut raw_response = String::new();

        for event in &events {
            // Check for "result" type event
            if event.event_type == "result" {
                if let Some(result) = &event.result {
                    raw_response = result.clone();
                    break;
                }
            }
            // Also check for assistant messages with content
            if event.event_type == "assistant" {
                if let Some(msg) = &event.message {
                    if let Some(content) = &msg.content {
                        raw_response.push_str(content);
                    }
                }
            }
        }

        let parsed = Self::extract_embedded_json(&raw_response);

        Ok(AgentResponse {
            files_created: parsed.files_created,
            files_modified: parsed.files_modified,
            commands_run: parsed.commands_run,
            raw_response,
        })
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
        assert!(response.raw_response.contains("made some changes"));
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
        assert!(response.raw_response.contains("broken JSON"));
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
    fn test_parse_preserves_raw_response() {
        let raw = r#"{
            "result": "This is the full response text with all details."
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert_eq!(response.raw_response, "This is the full response text with all details.");
    }

    #[test]
    fn test_parse_with_empty_result() {
        let raw = r#"{
            "result": ""
        }"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert!(response.raw_response.is_empty());
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
        assert!(response.raw_response.contains("I created the file"));
    }

    #[test]
    fn test_parse_streaming_format_with_assistant_message() {
        let raw = r#"[
            {"type":"system","subtype":"init","session_id":"abc123"},
            {"type":"assistant","message":{"content":"Done! Created src/bar.rs"}}
        ]"#;

        let response = ResponseParser::parse(raw).unwrap();

        assert!(response.raw_response.contains("Done!"));
    }
}
