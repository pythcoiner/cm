//! Validation utilities for tasks.json and roadmap.json.
//!
//! This module provides comprehensive validation of cm project state files,
//! including JSON syntax checking, schema validation, duplicate detection,
//! and cross-reference verification.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::Value;
use thiserror::Error;

/// Errors that can occur during sanity checking of state files.
#[derive(Debug, Clone, Error)]
pub enum SanityError {
    /// Invalid JSON syntax in a file.
    #[error("JSON syntax error in {file}: {message}")]
    JsonSyntaxError {
        /// The file containing the error.
        file: String,
        /// Description of the syntax error.
        message: String,
    },

    /// Missing or invalid field in the schema.
    #[error("Schema error in {file}: field '{field}' - {message}")]
    SchemaError {
        /// The file containing the error.
        file: String,
        /// The field that is missing or invalid.
        field: String,
        /// Description of the error.
        message: String,
    },

    /// Duplicate ID found in a file.
    #[error("Duplicate ID in {file}: '{id}'")]
    DuplicateId {
        /// The file containing the duplicate.
        file: String,
        /// The duplicate ID.
        id: String,
    },

    /// Cross-reference points to non-existent ID.
    #[error("Invalid reference from {source_file}:{source_id} to {target_file}:{target_id}")]
    InvalidReference {
        /// The file containing the reference.
        source_file: String,
        /// The ID of the item containing the reference.
        source_id: String,
        /// The file being referenced.
        target_file: String,
        /// The ID being referenced that does not exist.
        target_id: String,
    },

    /// ID exists but nothing references it (warning).
    #[error("Orphaned reference in {file}: ID '{id}' - {message}")]
    OrphanedReference {
        /// The file containing the orphaned item.
        file: String,
        /// The orphaned ID.
        id: String,
        /// Additional context about the orphan.
        message: String,
    },
}

/// Result of validating one or more state files.
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Critical errors that must be fixed.
    pub errors: Vec<SanityError>,
    /// Non-critical warnings (e.g., orphaned references).
    pub warnings: Vec<SanityError>,
}

impl ValidationResult {
    /// Create a new empty validation result.
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Merge another validation result into this one.
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    /// Add an error to the result.
    pub fn add_error(&mut self, error: SanityError) {
        self.errors.push(error);
    }

    /// Add a warning to the result.
    pub fn add_warning(&mut self, warning: SanityError) {
        self.warnings.push(warning);
    }
}

/// Validate the tasks.json file at the given path.
///
/// Checks:
/// - JSON syntax is valid
/// - Required fields exist: version, project.name, project.description, phases
/// - Each phase has: id, name, status, tasks
/// - Each task has: id, name, type, status, context, instructions
/// - No duplicate phase IDs
/// - No duplicate task IDs within phases
/// - Each attempt has: attempt_number, agent_id, started_at, status
/// - Each attempt response has: message (or legacy raw_response)
/// - Each agent_history entry has: id, task_id, agent_type, started_at
/// - Each log_record entry has: id, timestamp, action, data
///
/// # Arguments
///
/// * `path` - Path to the tasks.json file
///
/// # Returns
///
/// A `ValidationResult` containing any errors or warnings found.
pub fn validate_tasks_json(path: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();
    let file_name = path.display().to_string();

    // Read and parse the file
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: file_name,
                message: format!("Failed to read file: {}", e),
            });
            return result;
        }
    };

    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: file_name,
                message: e.to_string(),
            });
            return result;
        }
    };

    // Check required top-level fields
    check_required_field(&json, "version", &file_name, &mut result);

    // Check project fields
    if let Some(project) = json.get("project") {
        check_required_field(project, "name", &format!("{} (project)", file_name), &mut result);
        check_required_field(project, "description", &format!("{} (project)", file_name), &mut result);
    } else {
        result.add_error(SanityError::SchemaError {
            file: file_name.clone(),
            field: "project".to_string(),
            message: "required field is missing".to_string(),
        });
    }

    // Check phases
    let phases = match json.get("phases") {
        Some(Value::Array(arr)) => arr,
        Some(_) => {
            result.add_error(SanityError::SchemaError {
                file: file_name.clone(),
                field: "phases".to_string(),
                message: "must be an array".to_string(),
            });
            return result;
        }
        None => {
            result.add_error(SanityError::SchemaError {
                file: file_name.clone(),
                field: "phases".to_string(),
                message: "required field is missing".to_string(),
            });
            return result;
        }
    };

    // Check for duplicate phase IDs
    let mut phase_ids = HashSet::new();

    for (phase_idx, phase) in phases.iter().enumerate() {
        let phase_context = format!("{} (phases[{}])", file_name, phase_idx);

        // Check phase required fields
        check_required_field(phase, "id", &phase_context, &mut result);
        check_required_field(phase, "name", &phase_context, &mut result);
        check_required_field(phase, "status", &phase_context, &mut result);

        // Check for duplicate phase ID
        if let Some(id) = phase.get("id").and_then(|v| v.as_str()) {
            if !phase_ids.insert(id.to_string()) {
                result.add_error(SanityError::DuplicateId {
                    file: file_name.clone(),
                    id: id.to_string(),
                });
            }
        }

        // Check tasks within phase
        let tasks = match phase.get("tasks") {
            Some(Value::Array(arr)) => arr,
            Some(_) => {
                result.add_error(SanityError::SchemaError {
                    file: file_name.clone(),
                    field: format!("phases[{}].tasks", phase_idx),
                    message: "must be an array".to_string(),
                });
                continue;
            }
            None => {
                result.add_error(SanityError::SchemaError {
                    file: file_name.clone(),
                    field: format!("phases[{}].tasks", phase_idx),
                    message: "required field is missing".to_string(),
                });
                continue;
            }
        };

        // Check for duplicate task IDs within phase
        let mut task_ids = HashSet::new();

        for (task_idx, task) in tasks.iter().enumerate() {
            let task_context = format!("{} (phases[{}].tasks[{}])", file_name, phase_idx, task_idx);

            // Check task required fields
            check_required_field(task, "id", &task_context, &mut result);
            check_required_field(task, "name", &task_context, &mut result);
            check_required_field(task, "type", &task_context, &mut result);
            check_required_field(task, "status", &task_context, &mut result);
            check_required_field(task, "context", &task_context, &mut result);
            check_required_field(task, "instructions", &task_context, &mut result);

            // Check for duplicate task ID
            if let Some(id) = task.get("id").and_then(|v| v.as_str()) {
                if !task_ids.insert(id.to_string()) {
                    result.add_error(SanityError::DuplicateId {
                        file: file_name.clone(),
                        id: id.to_string(),
                    });
                }
            }

            // Validate attempts array if present
            if let Some(Value::Array(attempts)) = task.get("attempts") {
                for (attempt_idx, attempt) in attempts.iter().enumerate() {
                    let attempt_context = format!(
                        "{} (phases[{}].tasks[{}].attempts[{}])",
                        file_name, phase_idx, task_idx, attempt_idx
                    );

                    check_required_field(attempt, "attempt_number", &attempt_context, &mut result);
                    check_required_field(attempt, "agent_id", &attempt_context, &mut result);
                    check_required_field(attempt, "started_at", &attempt_context, &mut result);
                    check_required_field(attempt, "status", &attempt_context, &mut result);

                    // Validate response if present and non-null
                    if let Some(response) = attempt.get("response") {
                        if !response.is_null() {
                            let resp_context = format!("{} (response)", attempt_context);
                            // Accept either "message" or legacy "raw_response"
                            if response.get("message").is_none()
                                && response.get("raw_response").is_none()
                            {
                                result.add_error(SanityError::SchemaError {
                                    file: resp_context,
                                    field: "message".to_string(),
                                    message: "required field is missing (also checked legacy 'raw_response')".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate agent_history if present
    if let Some(agent_history) = json.get("agent_history") {
        match agent_history {
            Value::Array(entries) => {
                for (idx, entry) in entries.iter().enumerate() {
                    let ctx = format!("{} (agent_history[{}])", file_name, idx);
                    check_required_field(entry, "id", &ctx, &mut result);
                    check_required_field(entry, "task_id", &ctx, &mut result);
                    check_required_field(entry, "agent_type", &ctx, &mut result);
                    check_required_field(entry, "started_at", &ctx, &mut result);
                }
            }
            _ => {
                result.add_error(SanityError::SchemaError {
                    file: file_name.clone(),
                    field: "agent_history".to_string(),
                    message: "must be an array".to_string(),
                });
            }
        }
    }

    // Validate log_records if present
    if let Some(log_records) = json.get("log_records") {
        match log_records {
            Value::Array(entries) => {
                for (idx, entry) in entries.iter().enumerate() {
                    let ctx = format!("{} (log_records[{}])", file_name, idx);
                    check_required_field(entry, "id", &ctx, &mut result);
                    check_required_field(entry, "timestamp", &ctx, &mut result);
                    check_required_field(entry, "action", &ctx, &mut result);
                    check_required_field(entry, "data", &ctx, &mut result);
                }
            }
            _ => {
                result.add_error(SanityError::SchemaError {
                    file: file_name.clone(),
                    field: "log_records".to_string(),
                    message: "must be an array".to_string(),
                });
            }
        }
    }

    result
}

/// Validate the roadmap.json file at the given path.
///
/// Checks:
/// - JSON syntax is valid
/// - Required fields exist: version, title, phases
/// - Each phase has: id, number, name, items
/// - Each item has: id, name, completed
/// - No duplicate phase IDs
/// - No duplicate item IDs
///
/// # Arguments
///
/// * `path` - Path to the roadmap.json file
///
/// # Returns
///
/// A `ValidationResult` containing any errors or warnings found.
pub fn validate_roadmap_json(path: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();
    let file_name = path.display().to_string();

    // Read and parse the file
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: file_name,
                message: format!("Failed to read file: {}", e),
            });
            return result;
        }
    };

    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: file_name,
                message: e.to_string(),
            });
            return result;
        }
    };

    // Check required top-level fields
    check_required_field(&json, "version", &file_name, &mut result);
    check_required_field(&json, "title", &file_name, &mut result);

    // Check phases
    let phases = match json.get("phases") {
        Some(Value::Array(arr)) => arr,
        Some(_) => {
            result.add_error(SanityError::SchemaError {
                file: file_name.clone(),
                field: "phases".to_string(),
                message: "must be an array".to_string(),
            });
            return result;
        }
        None => {
            result.add_error(SanityError::SchemaError {
                file: file_name.clone(),
                field: "phases".to_string(),
                message: "required field is missing".to_string(),
            });
            return result;
        }
    };

    // Check for duplicate phase IDs and item IDs
    let mut phase_ids = HashSet::new();
    let mut item_ids = HashSet::new();

    for (phase_idx, phase) in phases.iter().enumerate() {
        let phase_context = format!("{} (phases[{}])", file_name, phase_idx);

        // Check phase required fields
        check_required_field(phase, "id", &phase_context, &mut result);
        check_required_field(phase, "number", &phase_context, &mut result);
        check_required_field(phase, "name", &phase_context, &mut result);

        // Check for duplicate phase ID
        if let Some(id) = phase.get("id").and_then(|v| v.as_str()) {
            if !phase_ids.insert(id.to_string()) {
                result.add_error(SanityError::DuplicateId {
                    file: file_name.clone(),
                    id: id.to_string(),
                });
            }
        }

        // Check items within phase
        let items = match phase.get("items") {
            Some(Value::Array(arr)) => arr,
            Some(_) => {
                result.add_error(SanityError::SchemaError {
                    file: file_name.clone(),
                    field: format!("phases[{}].items", phase_idx),
                    message: "must be an array".to_string(),
                });
                continue;
            }
            None => {
                result.add_error(SanityError::SchemaError {
                    file: file_name.clone(),
                    field: format!("phases[{}].items", phase_idx),
                    message: "required field is missing".to_string(),
                });
                continue;
            }
        };

        for (item_idx, item) in items.iter().enumerate() {
            let item_context = format!("{} (phases[{}].items[{}])", file_name, phase_idx, item_idx);

            // Check item required fields
            check_required_field(item, "id", &item_context, &mut result);
            check_required_field(item, "name", &item_context, &mut result);
            check_required_field(item, "completed", &item_context, &mut result);

            // Check for duplicate item ID
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                if !item_ids.insert(id.to_string()) {
                    result.add_error(SanityError::DuplicateId {
                        file: file_name.clone(),
                        id: id.to_string(),
                    });
                }
            }
        }
    }

    result
}

/// Validate cross-references between tasks.json and roadmap.json.
///
/// Checks:
/// - Each Task with roadmap_item_id references an existing roadmap item
/// - Each RoadmapItem with linked_task_ids references existing tasks
///
/// # Arguments
///
/// * `tasks_path` - Path to the tasks.json file
/// * `roadmap_path` - Path to the roadmap.json file
///
/// # Returns
///
/// A `ValidationResult` containing any errors or warnings found.
pub fn validate_cross_references(tasks_path: &Path, roadmap_path: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();
    let tasks_file = tasks_path.display().to_string();
    let roadmap_file = roadmap_path.display().to_string();

    // Load tasks.json
    let tasks_content = match fs::read_to_string(tasks_path) {
        Ok(c) => c,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: tasks_file,
                message: format!("Failed to read file: {}", e),
            });
            return result;
        }
    };

    let tasks_json: Value = match serde_json::from_str(&tasks_content) {
        Ok(v) => v,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: tasks_file,
                message: e.to_string(),
            });
            return result;
        }
    };

    // Load roadmap.json
    let roadmap_content = match fs::read_to_string(roadmap_path) {
        Ok(c) => c,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: roadmap_file,
                message: format!("Failed to read file: {}", e),
            });
            return result;
        }
    };

    let roadmap_json: Value = match serde_json::from_str(&roadmap_content) {
        Ok(v) => v,
        Err(e) => {
            result.add_error(SanityError::JsonSyntaxError {
                file: roadmap_file,
                message: e.to_string(),
            });
            return result;
        }
    };

    // Collect all task IDs
    let task_ids: HashSet<String> = collect_task_ids(&tasks_json);

    // Collect all roadmap item IDs
    let roadmap_item_ids: HashSet<String> = collect_roadmap_item_ids(&roadmap_json);

    // Check task -> roadmap references
    if let Some(Value::Array(phases)) = tasks_json.get("phases") {
        for phase in phases {
            if let Some(Value::Array(tasks)) = phase.get("tasks") {
                for task in tasks {
                    if let Some(roadmap_item_id) = task.get("roadmap_item_id").and_then(|v| v.as_str()) {
                        if !roadmap_item_ids.contains(roadmap_item_id) {
                            let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                            result.add_error(SanityError::InvalidReference {
                                source_file: tasks_file.clone(),
                                source_id: task_id.to_string(),
                                target_file: roadmap_file.clone(),
                                target_id: roadmap_item_id.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check roadmap -> task references
    if let Some(Value::Array(phases)) = roadmap_json.get("phases") {
        for phase in phases {
            if let Some(Value::Array(items)) = phase.get("items") {
                for item in items {
                    if let Some(Value::Array(linked_ids)) = item.get("linked_task_ids") {
                        let item_id = item.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                        for linked_id in linked_ids {
                            if let Some(linked_task_id) = linked_id.as_str() {
                                if !task_ids.contains(linked_task_id) {
                                    result.add_error(SanityError::InvalidReference {
                                        source_file: roadmap_file.clone(),
                                        source_id: item_id.to_string(),
                                        target_file: tasks_file.clone(),
                                        target_id: linked_task_id.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

/// Validate all state files in a cm directory.
///
/// Runs all validation checks:
/// - validate_tasks_json on cm_dir/tasks.json
/// - validate_roadmap_json on cm_dir/roadmap.json
/// - validate_cross_references between both files
///
/// # Arguments
///
/// * `cm_dir` - Path to the .cm directory
///
/// # Returns
///
/// A `ValidationResult` containing all errors and warnings from all checks.
pub fn validate_all(cm_dir: &Path) -> ValidationResult {
    let mut result = ValidationResult::new();

    let tasks_path = cm_dir.join("tasks.json");
    let roadmap_path = cm_dir.join("roadmap.json");

    // Validate tasks.json if it exists
    if tasks_path.exists() {
        result.merge(validate_tasks_json(&tasks_path));
    }

    // Validate roadmap.json if it exists
    if roadmap_path.exists() {
        result.merge(validate_roadmap_json(&roadmap_path));
    }

    // Validate cross-references if both files exist
    if tasks_path.exists() && roadmap_path.exists() {
        result.merge(validate_cross_references(&tasks_path, &roadmap_path));
    }

    result
}

/// Helper function to check if a required field exists in a JSON object.
fn check_required_field(json: &Value, field: &str, context: &str, result: &mut ValidationResult) {
    if json.get(field).is_none() {
        result.add_error(SanityError::SchemaError {
            file: context.to_string(),
            field: field.to_string(),
            message: "required field is missing".to_string(),
        });
    }
}

/// Collect all task IDs from a tasks.json Value.
fn collect_task_ids(json: &Value) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(Value::Array(phases)) = json.get("phases") {
        for phase in phases {
            if let Some(Value::Array(tasks)) = phase.get("tasks") {
                for task in tasks {
                    if let Some(id) = task.get("id").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
    }
    ids
}

/// Collect all roadmap item IDs from a roadmap.json Value.
fn collect_roadmap_item_ids(json: &Value) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(Value::Array(phases)) = json.get("phases") {
        for phase in phases {
            if let Some(Value::Array(items)) = phase.get("items") {
                for item in items {
                    if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_valid_tasks_json() -> String {
        r#"{
            "version": "1.0.0",
            "project": {
                "name": "Test Project",
                "description": "A test project"
            },
            "phases": [
                {
                    "id": "phase-1",
                    "name": "Phase One",
                    "status": "pending",
                    "tasks": [
                        {
                            "id": "task-1",
                            "name": "First Task",
                            "type": "implement",
                            "status": "pending",
                            "context": {},
                            "instructions": "Do something"
                        }
                    ]
                }
            ]
        }"#.to_string()
    }

    fn create_valid_roadmap_json() -> String {
        r#"{
            "version": "1.0.0",
            "title": "Test Roadmap",
            "phases": [
                {
                    "id": "phase-1",
                    "number": "1",
                    "name": "Phase One",
                    "items": [
                        {
                            "id": "item-1",
                            "name": "First Item",
                            "completed": false
                        }
                    ]
                }
            ]
        }"#.to_string()
    }

    #[test]
    fn test_valid_tasks_json_passes() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", create_valid_tasks_json()).unwrap();

        let result = validate_tasks_json(file.path());

        assert!(result.is_valid(), "Expected no errors, got: {:?}", result.errors);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_valid_roadmap_json_passes() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", create_valid_roadmap_json()).unwrap();

        let result = validate_roadmap_json(file.path());

        assert!(result.is_valid(), "Expected no errors, got: {:?}", result.errors);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_invalid_json_syntax() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{{ invalid json }}").unwrap();

        let result = validate_tasks_json(file.path());

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(&result.errors[0], SanityError::JsonSyntaxError { .. }));
    }

    #[test]
    fn test_missing_required_field() {
        let mut file = NamedTempFile::new().unwrap();
        // Missing project.name
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{
                "description": "A test project"
            }},
            "phases": []
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());

        assert!(!result.is_valid());
        let has_name_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::SchemaError { field, .. } if field == "name")
        });
        assert!(has_name_error, "Expected error for missing 'name' field, got: {:?}", result.errors);
    }

    #[test]
    fn test_duplicate_task_id() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{
                "name": "Test",
                "description": "Test"
            }},
            "phases": [
                {{
                    "id": "phase-1",
                    "name": "Phase",
                    "status": "pending",
                    "tasks": [
                        {{
                            "id": "duplicate-id",
                            "name": "Task 1",
                            "type": "implement",
                            "status": "pending",
                            "context": {{}},
                            "instructions": "Do something"
                        }},
                        {{
                            "id": "duplicate-id",
                            "name": "Task 2",
                            "type": "implement",
                            "status": "pending",
                            "context": {{}},
                            "instructions": "Do something else"
                        }}
                    ]
                }}
            ]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());

        assert!(!result.is_valid());
        let has_duplicate_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::DuplicateId { id, .. } if id == "duplicate-id")
        });
        assert!(has_duplicate_error, "Expected duplicate ID error, got: {:?}", result.errors);
    }

    #[test]
    fn test_duplicate_phase_id() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{
                "name": "Test",
                "description": "Test"
            }},
            "phases": [
                {{
                    "id": "same-phase",
                    "name": "Phase 1",
                    "status": "pending",
                    "tasks": []
                }},
                {{
                    "id": "same-phase",
                    "name": "Phase 2",
                    "status": "pending",
                    "tasks": []
                }}
            ]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());

        assert!(!result.is_valid());
        let has_duplicate_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::DuplicateId { id, .. } if id == "same-phase")
        });
        assert!(has_duplicate_error, "Expected duplicate phase ID error, got: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_cross_reference_task_to_roadmap() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let roadmap_path = dir.path().join("roadmap.json");

        // Tasks file with reference to non-existent roadmap item
        fs::write(&tasks_path, r#"{
            "version": "1.0.0",
            "project": { "name": "Test", "description": "Test" },
            "phases": [{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "pending",
                    "context": {},
                    "instructions": "Do it",
                    "roadmap_item_id": "nonexistent-item"
                }]
            }]
        }"#).unwrap();

        // Roadmap file without the referenced item
        fs::write(&roadmap_path, r#"{
            "version": "1.0.0",
            "title": "Test",
            "phases": [{
                "id": "phase-1",
                "number": "1",
                "name": "Phase",
                "items": [{
                    "id": "item-1",
                    "name": "Item",
                    "completed": false
                }]
            }]
        }"#).unwrap();

        let result = validate_cross_references(&tasks_path, &roadmap_path);

        assert!(!result.is_valid());
        let has_invalid_ref = result.errors.iter().any(|e| {
            matches!(e, SanityError::InvalidReference { target_id, .. } if target_id == "nonexistent-item")
        });
        assert!(has_invalid_ref, "Expected invalid reference error, got: {:?}", result.errors);
    }

    #[test]
    fn test_invalid_cross_reference_roadmap_to_task() {
        let dir = TempDir::new().unwrap();
        let tasks_path = dir.path().join("tasks.json");
        let roadmap_path = dir.path().join("roadmap.json");

        // Tasks file
        fs::write(&tasks_path, r#"{
            "version": "1.0.0",
            "project": { "name": "Test", "description": "Test" },
            "phases": [{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "pending",
                    "context": {},
                    "instructions": "Do it"
                }]
            }]
        }"#).unwrap();

        // Roadmap file with reference to non-existent task
        fs::write(&roadmap_path, r#"{
            "version": "1.0.0",
            "title": "Test",
            "phases": [{
                "id": "phase-1",
                "number": "1",
                "name": "Phase",
                "items": [{
                    "id": "item-1",
                    "name": "Item",
                    "completed": false,
                    "linked_task_ids": ["nonexistent-task"]
                }]
            }]
        }"#).unwrap();

        let result = validate_cross_references(&tasks_path, &roadmap_path);

        assert!(!result.is_valid());
        let has_invalid_ref = result.errors.iter().any(|e| {
            matches!(e, SanityError::InvalidReference { target_id, .. } if target_id == "nonexistent-task")
        });
        assert!(has_invalid_ref, "Expected invalid reference error, got: {:?}", result.errors);
    }

    #[test]
    fn test_validate_all_integration() {
        let dir = TempDir::new().unwrap();
        let cm_dir = dir.path();

        // Create valid tasks.json
        fs::write(cm_dir.join("tasks.json"), r#"{
            "version": "1.0.0",
            "project": { "name": "Test", "description": "Test" },
            "phases": [{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "pending",
                    "context": {},
                    "instructions": "Do it",
                    "roadmap_item_id": "item-1"
                }]
            }]
        }"#).unwrap();

        // Create valid roadmap.json with matching reference
        fs::write(cm_dir.join("roadmap.json"), r#"{
            "version": "1.0.0",
            "title": "Test",
            "phases": [{
                "id": "phase-1",
                "number": "1",
                "name": "Phase",
                "items": [{
                    "id": "item-1",
                    "name": "Item",
                    "completed": false,
                    "linked_task_ids": ["task-1"]
                }]
            }]
        }"#).unwrap();

        let result = validate_all(cm_dir);

        assert!(result.is_valid(), "Expected no errors, got: {:?}", result.errors);
    }

    #[test]
    fn test_validate_all_with_errors() {
        let dir = TempDir::new().unwrap();
        let cm_dir = dir.path();

        // Create invalid tasks.json (missing project.name)
        fs::write(cm_dir.join("tasks.json"), r#"{
            "version": "1.0.0",
            "project": { "description": "Test" },
            "phases": []
        }"#).unwrap();

        // Create invalid roadmap.json (missing title)
        fs::write(cm_dir.join("roadmap.json"), r#"{
            "version": "1.0.0",
            "phases": []
        }"#).unwrap();

        let result = validate_all(cm_dir);

        assert!(!result.is_valid());
        // Should have errors from both files
        assert!(result.errors.len() >= 2, "Expected at least 2 errors, got: {:?}", result.errors);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ValidationResult::new();
        result1.add_error(SanityError::JsonSyntaxError {
            file: "file1".to_string(),
            message: "error1".to_string(),
        });
        result1.add_warning(SanityError::OrphanedReference {
            file: "file1".to_string(),
            id: "id1".to_string(),
            message: "warning1".to_string(),
        });

        let mut result2 = ValidationResult::new();
        result2.add_error(SanityError::JsonSyntaxError {
            file: "file2".to_string(),
            message: "error2".to_string(),
        });

        result1.merge(result2);

        assert_eq!(result1.errors.len(), 2);
        assert_eq!(result1.warnings.len(), 1);
    }

    #[test]
    fn test_validation_result_is_valid() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());

        result.add_warning(SanityError::OrphanedReference {
            file: "file".to_string(),
            id: "id".to_string(),
            message: "warning".to_string(),
        });
        // Warnings don't affect validity
        assert!(result.is_valid());

        result.add_error(SanityError::JsonSyntaxError {
            file: "file".to_string(),
            message: "error".to_string(),
        });
        // Errors do affect validity
        assert!(!result.is_valid());
    }

    #[test]
    fn test_roadmap_duplicate_item_id() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "title": "Test",
            "phases": [
                {{
                    "id": "phase-1",
                    "number": "1",
                    "name": "Phase",
                    "items": [
                        {{
                            "id": "dup-item",
                            "name": "Item 1",
                            "completed": false
                        }},
                        {{
                            "id": "dup-item",
                            "name": "Item 2",
                            "completed": false
                        }}
                    ]
                }}
            ]
        }}"#).unwrap();

        let result = validate_roadmap_json(file.path());

        assert!(!result.is_valid());
        let has_duplicate_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::DuplicateId { id, .. } if id == "dup-item")
        });
        assert!(has_duplicate_error, "Expected duplicate item ID error, got: {:?}", result.errors);
    }

    #[test]
    fn test_missing_tasks_file() {
        let dir = TempDir::new().unwrap();
        let nonexistent_path = dir.path().join("nonexistent.json");

        let result = validate_tasks_json(&nonexistent_path);

        assert!(!result.is_valid());
        assert!(matches!(&result.errors[0], SanityError::JsonSyntaxError { message, .. } if message.contains("Failed to read")));
    }

    #[test]
    fn test_valid_task_with_attempts() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [{{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "completed",
                    "context": {{}},
                    "instructions": "Do it",
                    "attempts": [{{
                        "attempt_number": 1,
                        "agent_id": "agent-1",
                        "started_at": "2026-01-24T14:07:20Z",
                        "status": "success",
                        "response": {{
                            "message": "Done"
                        }}
                    }}]
                }}]
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(result.is_valid(), "Expected no errors, got: {:?}", result.errors);
    }

    #[test]
    fn test_attempt_missing_required_field() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [{{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "completed",
                    "context": {{}},
                    "instructions": "Do it",
                    "attempts": [{{
                        "attempt_number": 1,
                        "started_at": "2026-01-24T14:07:20Z",
                        "status": "success"
                    }}]
                }}]
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(!result.is_valid());
        let has_agent_id_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::SchemaError { field, .. } if field == "agent_id")
        });
        assert!(has_agent_id_error, "Expected error for missing 'agent_id', got: {:?}", result.errors);
    }

    #[test]
    fn test_response_missing_message() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [{{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "completed",
                    "context": {{}},
                    "instructions": "Do it",
                    "attempts": [{{
                        "attempt_number": 1,
                        "agent_id": "agent-1",
                        "started_at": "2026-01-24T14:07:20Z",
                        "status": "success",
                        "response": {{
                            "files_created": []
                        }}
                    }}]
                }}]
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(!result.is_valid());
        let has_message_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::SchemaError { field, .. } if field == "message")
        });
        assert!(has_message_error, "Expected error for missing 'message', got: {:?}", result.errors);
    }

    #[test]
    fn test_response_with_legacy_raw_response() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [{{
                "id": "phase-1",
                "name": "Phase",
                "status": "pending",
                "tasks": [{{
                    "id": "task-1",
                    "name": "Task",
                    "type": "implement",
                    "status": "completed",
                    "context": {{}},
                    "instructions": "Do it",
                    "attempts": [{{
                        "attempt_number": 1,
                        "agent_id": "agent-1",
                        "started_at": "2026-01-24T14:07:20Z",
                        "status": "success",
                        "response": {{
                            "raw_response": "Done via legacy field"
                        }}
                    }}]
                }}]
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(result.is_valid(), "Legacy raw_response should be accepted, got: {:?}", result.errors);
    }

    #[test]
    fn test_agent_history_validation() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [],
            "agent_history": [{{
                "id": "inv-1",
                "task_id": "task-1",
                "agent_type": "implem",
                "started_at": "2026-01-24T14:07:20Z"
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(result.is_valid(), "Expected no errors, got: {:?}", result.errors);
    }

    #[test]
    fn test_agent_history_missing_field() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [],
            "agent_history": [{{
                "id": "inv-1",
                "started_at": "2026-01-24T14:07:20Z"
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(!result.is_valid());
        let has_task_id_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::SchemaError { field, .. } if field == "task_id")
        });
        assert!(has_task_id_error, "Expected error for missing 'task_id', got: {:?}", result.errors);
    }

    #[test]
    fn test_log_records_validation() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [],
            "log_records": [{{
                "id": "rec-1",
                "timestamp": "2026-01-24T14:07:20Z",
                "action": "phase_start",
                "data": {{ "type": "phase_start", "name": "Phase 1" }}
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(result.is_valid(), "Expected no errors, got: {:?}", result.errors);
    }

    #[test]
    fn test_log_records_missing_field() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [],
            "log_records": [{{
                "id": "rec-1",
                "action": "phase_start"
            }}]
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(!result.is_valid());
        let has_timestamp_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::SchemaError { field, .. } if field == "timestamp")
        });
        assert!(has_timestamp_error, "Expected error for missing 'timestamp', got: {:?}", result.errors);
    }

    #[test]
    fn test_agent_history_not_array() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "version": "1.0.0",
            "project": {{ "name": "Test", "description": "Test" }},
            "phases": [],
            "agent_history": "not an array"
        }}"#).unwrap();

        let result = validate_tasks_json(file.path());
        assert!(!result.is_valid());
        let has_type_error = result.errors.iter().any(|e| {
            matches!(e, SanityError::SchemaError { field, message, .. }
                if field == "agent_history" && message == "must be an array")
        });
        assert!(has_type_error, "Expected 'must be an array' error, got: {:?}", result.errors);
    }
}
