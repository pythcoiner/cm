//! ROADMAP.md generation from RoadmapState.
//!
//! This module generates ROADMAP.md content from a RoadmapState.
//! The generation is deterministic: the same state always produces the same output.

use crate::state::{RoadmapItem, RoadmapPhase, RoadmapState, RoadmapSubItem};

/// Generate ROADMAP.md content from a roadmap state.
///
/// The generated markdown uses checkboxes to indicate completion status:
/// - `[x]` for completed items
/// - `[ ]` for pending items
///
/// # Arguments
///
/// * `roadmap` - The roadmap state to generate markdown from
///
/// # Returns
///
/// A string containing the complete ROADMAP.md content.
pub fn generate_roadmap_md(roadmap: &RoadmapState) -> String {
    let mut output = String::new();

    // Title
    output.push_str(&format!("# {} - Roadmap\n\n", roadmap.title));
    output.push_str(
        "This document tracks implementation progress. Check off items as they are completed.\n\n",
    );

    // Phases
    for phase in &roadmap.phases {
        output.push_str(&format_phase(phase));
        output.push_str("\n---\n\n");
    }

    // Summary table
    output.push_str(&generate_summary_table(roadmap));

    output
}

/// Format a single phase as markdown.
fn format_phase(phase: &RoadmapPhase) -> String {
    let mut output = String::new();

    // Phase header
    output.push_str(&format!("## Phase {}: {}\n\n", phase.number, phase.name));

    // Phase status
    let completed_items = phase.items.iter().filter(|i| i.completed).count();
    let total_items = phase.items.len();
    let status = if completed_items == total_items && total_items > 0 {
        "Complete"
    } else if completed_items > 0 {
        "In Progress"
    } else {
        "Not Started"
    };
    output.push_str(&format!(
        "Status: **{status}** ({completed_items}/{total_items})\n\n"
    ));

    // Items
    for item in &phase.items {
        output.push_str(&format_item(item));
    }

    output
}

/// Format a single item as markdown.
fn format_item(item: &RoadmapItem) -> String {
    let mut output = String::new();

    // Main item checkbox
    let checkbox = if item.completed { "[x]" } else { "[ ]" };
    output.push_str(&format!("- {} {}\n", checkbox, item.name));

    // Sub-items (indented)
    for sub_item in &item.sub_items {
        output.push_str(&format_sub_item(sub_item));
    }

    output
}

/// Format a sub-item as markdown.
fn format_sub_item(sub_item: &RoadmapSubItem) -> String {
    let checkbox = if sub_item.completed { "[x]" } else { "[ ]" };
    format!("  - {} {}\n", checkbox, sub_item.name)
}

/// Generate the summary table.
fn generate_summary_table(roadmap: &RoadmapState) -> String {
    let mut output = String::new();

    output.push_str("## Summary\n\n");
    output.push_str("| Phase | Status | Progress |\n");
    output.push_str("|-------|--------|----------|\n");

    let mut total_completed = 0;
    let mut total_items = 0;

    for phase in &roadmap.phases {
        let completed = count_completed_in_phase(phase);
        let items = count_items_in_phase(phase);
        total_completed += completed;
        total_items += items;

        let status = if completed == items && items > 0 {
            "Complete"
        } else if completed > 0 {
            "In Progress"
        } else {
            "Not Started"
        };

        output.push_str(&format!(
            "| Phase {}: {} | {} | {}/{} |\n",
            phase.number, phase.name, status, completed, items
        ));
    }

    // Total row
    output.push_str(&format!(
        "| **Total** | | **{total_completed}/{total_items}** |\n"
    ));

    output
}

/// Count completed items in a phase (including sub-items).
fn count_completed_in_phase(phase: &RoadmapPhase) -> usize {
    phase
        .items
        .iter()
        .map(|item| {
            let item_complete = if item.completed { 1 } else { 0 };
            let sub_complete = item.sub_items.iter().filter(|s| s.completed).count();
            item_complete + sub_complete
        })
        .sum()
}

/// Count total items in a phase (including sub-items).
fn count_items_in_phase(phase: &RoadmapPhase) -> usize {
    phase
        .items
        .iter()
        .map(|item| 1 + item.sub_items.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_roadmap() -> RoadmapState {
        RoadmapState {
            version: "1.0.0".to_string(),
            title: "Test Project".to_string(),
            phases: vec![
                RoadmapPhase {
                    id: "phase-1".to_string(),
                    number: "1".to_string(),
                    name: "Setup".to_string(),
                    items: vec![
                        RoadmapItem {
                            id: "item-1".to_string(),
                            name: "Initialize project".to_string(),
                            completed: true,
                            sub_items: vec![
                                RoadmapSubItem {
                                    id: Some("item-1.sub-0".to_string()),
                                    name: "Create Cargo.toml".to_string(),
                                    completed: true,
                                },
                                RoadmapSubItem {
                                    id: Some("item-1.sub-1".to_string()),
                                    name: "Add dependencies".to_string(),
                                    completed: true,
                                },
                            ],
                            linked_task_ids: vec!["phase-1.task-1".to_string()],
                        },
                        RoadmapItem {
                            id: "item-2".to_string(),
                            name: "Create module structure".to_string(),
                            completed: false,
                            sub_items: vec![],
                            linked_task_ids: vec![],
                        },
                    ],
                },
                RoadmapPhase {
                    id: "phase-2".to_string(),
                    number: "2".to_string(),
                    name: "Implementation".to_string(),
                    items: vec![RoadmapItem {
                        id: "item-3".to_string(),
                        name: "Core logic".to_string(),
                        completed: false,
                        sub_items: vec![],
                        linked_task_ids: vec![],
                    }],
                },
            ],
        }
    }

    #[test]
    fn test_generate_roadmap_md() {
        let roadmap = create_test_roadmap();
        let output = generate_roadmap_md(&roadmap);

        // Check header
        assert!(output.contains("# Test Project - Roadmap"));
        assert!(output.contains("tracks implementation progress"));

        // Check phase headers
        assert!(output.contains("## Phase 1: Setup"));
        assert!(output.contains("## Phase 2: Implementation"));

        // Check checkboxes
        assert!(output.contains("- [x] Initialize project"));
        assert!(output.contains("- [ ] Create module structure"));
        assert!(output.contains("  - [x] Create Cargo.toml"));
        assert!(output.contains("  - [x] Add dependencies"));

        // Check summary table
        assert!(output.contains("## Summary"));
        assert!(output.contains("| Phase | Status | Progress |"));
    }

    #[test]
    fn test_format_phase() {
        let phase = RoadmapPhase {
            id: "phase-1".to_string(),
            number: "1".to_string(),
            name: "Test Phase".to_string(),
            items: vec![
                RoadmapItem {
                    id: "item-1".to_string(),
                    name: "Item One".to_string(),
                    completed: true,
                    sub_items: vec![],
                    linked_task_ids: vec![],
                },
                RoadmapItem {
                    id: "item-2".to_string(),
                    name: "Item Two".to_string(),
                    completed: false,
                    sub_items: vec![],
                    linked_task_ids: vec![],
                },
            ],
        };

        let output = format_phase(&phase);
        assert!(output.contains("## Phase 1: Test Phase"));
        assert!(output.contains("Status: **In Progress** (1/2)"));
        assert!(output.contains("- [x] Item One"));
        assert!(output.contains("- [ ] Item Two"));
    }

    #[test]
    fn test_format_item_with_sub_items() {
        let item = RoadmapItem {
            id: "item-1".to_string(),
            name: "Main Item".to_string(),
            completed: false,
            sub_items: vec![
                RoadmapSubItem {
                    id: Some("item-1.sub-0".to_string()),
                    name: "Sub A".to_string(),
                    completed: true,
                },
                RoadmapSubItem {
                    id: Some("item-1.sub-1".to_string()),
                    name: "Sub B".to_string(),
                    completed: false,
                },
            ],
            linked_task_ids: vec![],
        };

        let output = format_item(&item);
        assert!(output.contains("- [ ] Main Item"));
        assert!(output.contains("  - [x] Sub A"));
        assert!(output.contains("  - [ ] Sub B"));
    }

    #[test]
    fn test_count_completed_in_phase() {
        let phase = RoadmapPhase {
            id: "phase-1".to_string(),
            number: "1".to_string(),
            name: "Test".to_string(),
            items: vec![RoadmapItem {
                id: "item-1".to_string(),
                name: "Item".to_string(),
                completed: true,
                sub_items: vec![
                    RoadmapSubItem {
                        id: Some("item-1.sub-0".to_string()),
                        name: "A".to_string(),
                        completed: true,
                    },
                    RoadmapSubItem {
                        id: Some("item-1.sub-1".to_string()),
                        name: "B".to_string(),
                        completed: false,
                    },
                ],
                linked_task_ids: vec![],
            }],
        };

        // 1 item completed + 1 sub-item completed = 2
        assert_eq!(count_completed_in_phase(&phase), 2);
    }

    #[test]
    fn test_count_items_in_phase() {
        let phase = RoadmapPhase {
            id: "phase-1".to_string(),
            number: "1".to_string(),
            name: "Test".to_string(),
            items: vec![RoadmapItem {
                id: "item-1".to_string(),
                name: "Item".to_string(),
                completed: true,
                sub_items: vec![
                    RoadmapSubItem {
                        id: Some("item-1.sub-0".to_string()),
                        name: "A".to_string(),
                        completed: true,
                    },
                    RoadmapSubItem {
                        id: Some("item-1.sub-1".to_string()),
                        name: "B".to_string(),
                        completed: false,
                    },
                ],
                linked_task_ids: vec![],
            }],
        };

        // 1 item + 2 sub-items = 3
        assert_eq!(count_items_in_phase(&phase), 3);
    }

    #[test]
    fn test_summary_table() {
        let roadmap = create_test_roadmap();
        let output = generate_summary_table(&roadmap);

        assert!(output.contains("| Phase 1: Setup |"));
        assert!(output.contains("| Phase 2: Implementation |"));
        assert!(output.contains("| **Total** |"));
    }

    #[test]
    fn test_empty_roadmap() {
        let roadmap = RoadmapState::new("Empty Project".to_string());
        let output = generate_roadmap_md(&roadmap);

        assert!(output.contains("# Empty Project - Roadmap"));
        assert!(output.contains("## Summary"));
        assert!(output.contains("| **Total** | | **0/0** |"));
    }
}
