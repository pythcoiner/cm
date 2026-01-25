//! Embedded command files for cm init.

pub const CM_COMMAND: &str = include_str!("../assets/cm.md");
pub const FEAT_COMMAND: &str = include_str!("../assets/feat.md");
pub const FIX_COMMAND: &str = include_str!("../assets/fix.md");
pub const END_COMMAND: &str = include_str!("../assets/end.md");

// Template files for cm init
pub const MANAGER_TEMPLATE: &str = include_str!("../assets/templates/MANAGER.md");
pub const IMPLEMENTER_TEMPLATE: &str = include_str!("../assets/templates/IMPLEMENTER.md");
pub const REVIEWER_TEMPLATE: &str = include_str!("../assets/templates/REVIEWER.md");
pub const FIX_TEMPLATE: &str = include_str!("../assets/templates/FIX.md");
pub const STRUCTURE_TEMPLATE: &str = include_str!("../assets/templates/STRUCTURE.md");
pub const ACTIONS_TEMPLATE: &str = include_str!("../assets/templates/ACTIONS.md");
pub const PLAN_TEMPLATE: &str = include_str!("../assets/templates/PLAN.md");
