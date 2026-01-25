//! RoadmapState types for structured roadmap storage.
//!
//! This module provides types for storing roadmap data as structured JSON.
//! The roadmap.json is the source of truth for ROADMAP.md generation.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::StateError;

/// The root structure for roadmap.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapState {
    /// Schema version for roadmap.json format.
    pub version: String,
    /// Title of the roadmap (usually project name).
    pub title: String,
    /// List of phases in the roadmap.
    pub phases: Vec<RoadmapPhase>,
}

/// A phase in the roadmap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapPhase {
    /// Unique identifier for this phase.
    pub id: String,
    /// Phase number (e.g., "0.5", "1", "2").
    pub number: String,
    /// Human-readable name for the phase.
    pub name: String,
    /// Items within this phase.
    pub items: Vec<RoadmapItem>,
}

/// An item within a roadmap phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapItem {
    /// Unique identifier for this item.
    pub id: String,
    /// Human-readable name for the item.
    pub name: String,
    /// Whether this item is completed.
    pub completed: bool,
    /// Sub-items within this item.
    #[serde(default)]
    pub sub_items: Vec<RoadmapSubItem>,
    /// IDs of tasks linked to this item.
    #[serde(default)]
    pub linked_task_ids: Vec<String>,
}

/// A sub-item within a roadmap item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapSubItem {
    /// Unique identifier for the sub-item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Human-readable name for the sub-item.
    pub name: String,
    /// Whether this sub-item is completed.
    pub completed: bool,
}

/// Load the roadmap state from a JSON file.
///
/// After loading, this function automatically calls `ensure_subitem_ids()` to
/// generate IDs for any sub-items that don't have them. This ensures backward
/// compatibility with older roadmap.json files.
///
/// # Arguments
///
/// * `path` - Path to the roadmap.json file
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist (`StateError::NotFound`)
/// - The file cannot be read (`StateError::Io`)
/// - The JSON is invalid (`StateError::ParseError`)
pub fn load_roadmap(path: &Path) -> Result<RoadmapState, StateError> {
    let content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            StateError::NotFound(path.to_path_buf())
        } else {
            StateError::Io(e)
        }
    })?;

    let mut roadmap: RoadmapState =
        serde_json::from_str(&content).map_err(|e| StateError::ParseError(e.to_string()))?;

    // Auto-generate IDs for any sub-items that don't have them
    roadmap.ensure_subitem_ids();

    Ok(roadmap)
}

/// Save the roadmap state to a JSON file.
///
/// Creates parent directories if they don't exist.
///
/// # Arguments
///
/// * `roadmap` - The roadmap state to save
/// * `path` - Path to write the roadmap.json file
///
/// # Errors
///
/// Returns an error if:
/// - Parent directories cannot be created (`StateError::Io`)
/// - The file cannot be written (`StateError::Io`)
/// - Serialization fails (`StateError::ParseError`)
pub fn save_roadmap(roadmap: &RoadmapState, path: &Path) -> Result<(), StateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content =
        serde_json::to_string_pretty(roadmap).map_err(|e| StateError::ParseError(e.to_string()))?;

    fs::write(path, content)?;
    Ok(())
}

impl RoadmapState {
    /// Create a new empty roadmap state.
    pub fn new(title: String) -> Self {
        Self {
            version: "1.0.0".to_string(),
            title,
            phases: vec![],
        }
    }

    /// Get the total number of items (including sub-items).
    pub fn total_items(&self) -> usize {
        self.phases
            .iter()
            .flat_map(|p| &p.items)
            .map(|item| 1 + item.sub_items.len())
            .sum()
    }

    /// Get the number of completed items (including sub-items).
    pub fn completed_items(&self) -> usize {
        self.phases
            .iter()
            .flat_map(|p| &p.items)
            .map(|item| {
                let item_complete = if item.completed { 1 } else { 0 };
                let sub_complete = item.sub_items.iter().filter(|s| s.completed).count();
                item_complete + sub_complete
            })
            .sum()
    }

    /// Ensure all sub-items have unique IDs.
    ///
    /// For any sub-item with `id: None`, generates an ID using the format
    /// `"{item_id}.sub-{index}"` (e.g., "27.4.sub-0").
    ///
    /// This method is called automatically by `load_roadmap()` to ensure
    /// backward compatibility with older roadmap.json files that don't have
    /// sub-item IDs.
    ///
    /// # Returns
    ///
    /// Mutable reference to self for chaining.
    pub fn ensure_subitem_ids(&mut self) -> &mut Self {
        for phase in &mut self.phases {
            for item in &mut phase.items {
                for (index, sub_item) in item.sub_items.iter_mut().enumerate() {
                    if sub_item.id.is_none() {
                        sub_item.id = Some(format!("{}.sub-{}", item.id, index));
                    }
                }
            }
        }
        self
    }

    /// Find a sub-item by its ID and return a mutable reference.
    ///
    /// Searches through all phases, items, and sub-items to find a sub-item
    /// with the given ID.
    ///
    /// # Arguments
    ///
    /// * `subitem_id` - The ID of the sub-item to find
    ///
    /// # Returns
    ///
    /// A mutable reference to the sub-item if found, or `None` if not found.
    pub fn find_subitem_by_id_mut(&mut self, subitem_id: &str) -> Option<&mut RoadmapSubItem> {
        for phase in &mut self.phases {
            for item in &mut phase.items {
                for sub_item in &mut item.sub_items {
                    if sub_item.id.as_deref() == Some(subitem_id) {
                        return Some(sub_item);
                    }
                }
            }
        }
        None
    }

    /// Find a sub-item by its ID and return an immutable reference.
    ///
    /// Searches through all phases, items, and sub-items to find a sub-item
    /// with the given ID.
    ///
    /// # Arguments
    ///
    /// * `subitem_id` - The ID of the sub-item to find
    ///
    /// # Returns
    ///
    /// An immutable reference to the sub-item if found, or `None` if not found.
    pub fn find_subitem_by_id(&self, subitem_id: &str) -> Option<&RoadmapSubItem> {
        for phase in &self.phases {
            for item in &phase.items {
                for sub_item in &item.sub_items {
                    if sub_item.id.as_deref() == Some(subitem_id) {
                        return Some(sub_item);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_roadmap() -> RoadmapState {
        RoadmapState {
            version: "1.0.0".to_string(),
            title: "Test Project".to_string(),
            phases: vec![
                RoadmapPhase {
                    id: "phase-1".to_string(),
                    number: "1".to_string(),
                    name: "Phase One".to_string(),
                    items: vec![
                        RoadmapItem {
                            id: "item-1".to_string(),
                            name: "First Item".to_string(),
                            completed: true,
                            sub_items: vec![
                                RoadmapSubItem {
                                    id: Some("item-1.sub-0".to_string()),
                                    name: "Sub-item A".to_string(),
                                    completed: true,
                                },
                                RoadmapSubItem {
                                    id: Some("item-1.sub-1".to_string()),
                                    name: "Sub-item B".to_string(),
                                    completed: false,
                                },
                            ],
                            linked_task_ids: vec!["phase-1.task-1".to_string()],
                        },
                        RoadmapItem {
                            id: "item-2".to_string(),
                            name: "Second Item".to_string(),
                            completed: false,
                            sub_items: vec![],
                            linked_task_ids: vec![],
                        },
                    ],
                },
                RoadmapPhase {
                    id: "phase-2".to_string(),
                    number: "2".to_string(),
                    name: "Phase Two".to_string(),
                    items: vec![RoadmapItem {
                        id: "item-3".to_string(),
                        name: "Third Item".to_string(),
                        completed: false,
                        sub_items: vec![],
                        linked_task_ids: vec![],
                    }],
                },
            ],
        }
    }

    #[test]
    fn test_roadmap_state_new() {
        let roadmap = RoadmapState::new("My Project".to_string());
        assert_eq!(roadmap.version, "1.0.0");
        assert_eq!(roadmap.title, "My Project");
        assert!(roadmap.phases.is_empty());
    }

    #[test]
    fn test_total_items() {
        let roadmap = create_test_roadmap();
        // 3 items + 2 sub-items = 5
        assert_eq!(roadmap.total_items(), 5);
    }

    #[test]
    fn test_completed_items() {
        let roadmap = create_test_roadmap();
        // item-1 is completed (1), sub-item A is completed (1) = 2
        assert_eq!(roadmap.completed_items(), 2);
    }

    #[test]
    fn test_load_save_round_trip() {
        let roadmap = create_test_roadmap();
        let file = NamedTempFile::new().unwrap();

        save_roadmap(&roadmap, file.path()).unwrap();

        let loaded = load_roadmap(file.path()).unwrap();

        assert_eq!(loaded.version, roadmap.version);
        assert_eq!(loaded.title, roadmap.title);
        assert_eq!(loaded.phases.len(), roadmap.phases.len());
        assert_eq!(loaded.phases[0].items.len(), 2);
        assert_eq!(loaded.phases[0].items[0].sub_items.len(), 2);
    }

    #[test]
    fn test_load_not_found() {
        let result = load_roadmap(Path::new("/nonexistent/roadmap.json"));
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[test]
    fn test_load_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{ invalid json }}").unwrap();

        let result = load_roadmap(file.path());
        assert!(matches!(result, Err(StateError::ParseError(_))));
    }

    #[test]
    fn test_roadmap_serialization() {
        let roadmap = create_test_roadmap();
        let json = serde_json::to_string_pretty(&roadmap).unwrap();

        assert!(json.contains("\"version\": \"1.0.0\""));
        assert!(json.contains("\"title\": \"Test Project\""));
        assert!(json.contains("\"number\": \"1\""));
        assert!(json.contains("\"completed\": true"));
        assert!(json.contains("\"linked_task_ids\""));
    }

    #[test]
    fn test_ensure_subitem_ids_generates_missing_ids() {
        let mut roadmap = RoadmapState {
            version: "1.0.0".to_string(),
            title: "Test".to_string(),
            phases: vec![RoadmapPhase {
                id: "phase-1".to_string(),
                number: "1".to_string(),
                name: "Phase One".to_string(),
                items: vec![RoadmapItem {
                    id: "27.4".to_string(),
                    name: "Item".to_string(),
                    completed: false,
                    sub_items: vec![
                        RoadmapSubItem {
                            id: None,
                            name: "First sub".to_string(),
                            completed: false,
                        },
                        RoadmapSubItem {
                            id: None,
                            name: "Second sub".to_string(),
                            completed: false,
                        },
                    ],
                    linked_task_ids: vec![],
                }],
            }],
        };

        roadmap.ensure_subitem_ids();

        assert_eq!(
            roadmap.phases[0].items[0].sub_items[0].id,
            Some("27.4.sub-0".to_string())
        );
        assert_eq!(
            roadmap.phases[0].items[0].sub_items[1].id,
            Some("27.4.sub-1".to_string())
        );
    }

    #[test]
    fn test_ensure_subitem_ids_preserves_existing_ids() {
        let mut roadmap = RoadmapState {
            version: "1.0.0".to_string(),
            title: "Test".to_string(),
            phases: vec![RoadmapPhase {
                id: "phase-1".to_string(),
                number: "1".to_string(),
                name: "Phase One".to_string(),
                items: vec![RoadmapItem {
                    id: "item-1".to_string(),
                    name: "Item".to_string(),
                    completed: false,
                    sub_items: vec![
                        RoadmapSubItem {
                            id: Some("custom-id".to_string()),
                            name: "First sub".to_string(),
                            completed: false,
                        },
                        RoadmapSubItem {
                            id: None,
                            name: "Second sub".to_string(),
                            completed: false,
                        },
                    ],
                    linked_task_ids: vec![],
                }],
            }],
        };

        roadmap.ensure_subitem_ids();

        // Existing ID should be preserved
        assert_eq!(
            roadmap.phases[0].items[0].sub_items[0].id,
            Some("custom-id".to_string())
        );
        // Missing ID should be generated
        assert_eq!(
            roadmap.phases[0].items[0].sub_items[1].id,
            Some("item-1.sub-1".to_string())
        );
    }

    #[test]
    fn test_ensure_subitem_ids_chaining() {
        let mut roadmap = RoadmapState::new("Test".to_string());
        // Should return &mut Self for chaining
        let result = roadmap.ensure_subitem_ids();
        assert_eq!(result.title, "Test");
    }

    #[test]
    fn test_find_subitem_by_id_found() {
        let roadmap = create_test_roadmap();
        let sub_item = roadmap.find_subitem_by_id("item-1.sub-0");
        assert!(sub_item.is_some());
        assert_eq!(sub_item.unwrap().name, "Sub-item A");
    }

    #[test]
    fn test_find_subitem_by_id_not_found() {
        let roadmap = create_test_roadmap();
        let sub_item = roadmap.find_subitem_by_id("nonexistent");
        assert!(sub_item.is_none());
    }

    #[test]
    fn test_find_subitem_by_id_mut_update() {
        let mut roadmap = create_test_roadmap();

        // Find and update the sub-item
        if let Some(sub_item) = roadmap.find_subitem_by_id_mut("item-1.sub-1") {
            sub_item.completed = true;
        }

        // Verify the update persisted
        let sub_item = roadmap.find_subitem_by_id("item-1.sub-1");
        assert!(sub_item.is_some());
        assert!(sub_item.unwrap().completed);
    }

    #[test]
    fn test_load_roadmap_auto_generates_subitem_ids() {
        let mut file = NamedTempFile::new().unwrap();
        // JSON without sub-item IDs (old format)
        writeln!(file, r#"{{
            "version": "1.0.0",
            "title": "Test",
            "phases": [{{
                "id": "phase-1",
                "number": "1",
                "name": "Phase",
                "items": [{{
                    "id": "item-1",
                    "name": "Item",
                    "completed": false,
                    "sub_items": [
                        {{ "name": "Sub A", "completed": false }},
                        {{ "name": "Sub B", "completed": true }}
                    ]
                }}]
            }}]
        }}"#).unwrap();

        let roadmap = load_roadmap(file.path()).unwrap();

        // IDs should have been auto-generated
        assert_eq!(
            roadmap.phases[0].items[0].sub_items[0].id,
            Some("item-1.sub-0".to_string())
        );
        assert_eq!(
            roadmap.phases[0].items[0].sub_items[1].id,
            Some("item-1.sub-1".to_string())
        );
    }

    #[test]
    fn test_subitem_id_not_serialized_when_none() {
        let sub_item = RoadmapSubItem {
            id: None,
            name: "Test".to_string(),
            completed: false,
        };
        let json = serde_json::to_string(&sub_item).unwrap();
        // ID field should not appear in output when None
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_subitem_id_serialized_when_present() {
        let sub_item = RoadmapSubItem {
            id: Some("test-id".to_string()),
            name: "Test".to_string(),
            completed: false,
        };
        let json = serde_json::to_string(&sub_item).unwrap();
        assert!(json.contains("\"id\":\"test-id\""));
    }
}
