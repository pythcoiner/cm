//! Manager state machine for tracking execution flow.
//!
//! This module defines the internal state of the manager during task execution,
//! allowing for proper state transitions and recovery.

/// The current state of the manager's execution loop.
///
/// This enum tracks where the manager is in the execution flow,
/// enabling proper state transitions and crash recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerState {
    /// Manager is idle, not currently executing any task.
    Idle,
    /// Manager is executing a task (preparing, selecting).
    Executing,
    /// Manager is waiting for an agent to complete.
    WaitingForAgent,
    /// Manager is verifying build results.
    Verifying,
}

impl Default for ManagerState {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for ManagerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagerState::Idle => write!(f, "Idle"),
            ManagerState::Executing => write!(f, "Executing"),
            ManagerState::WaitingForAgent => write!(f, "Waiting for Agent"),
            ManagerState::Verifying => write!(f, "Verifying"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_state_default() {
        let state = ManagerState::default();
        assert_eq!(state, ManagerState::Idle);
    }

    #[test]
    fn test_manager_state_display() {
        assert_eq!(ManagerState::Idle.to_string(), "Idle");
        assert_eq!(ManagerState::Executing.to_string(), "Executing");
        assert_eq!(
            ManagerState::WaitingForAgent.to_string(),
            "Waiting for Agent"
        );
        assert_eq!(ManagerState::Verifying.to_string(), "Verifying");
    }

    #[test]
    fn test_manager_state_copy() {
        let state1 = ManagerState::Executing;
        let state2 = state1;
        assert_eq!(state1, state2);
    }
}
