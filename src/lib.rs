pub mod agent;
pub mod collectors;
pub mod dashboard;
// Re-export modules so integration tests in tests/ can access them.
pub mod config;
pub mod git;
pub mod monitor;
pub mod path_utils;
pub mod scanner;
