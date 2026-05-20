//! Read-path helpers for Firefam memories.
//!
//! This crate owns memory injection, memory citation parsing, and telemetry
//! classification for read access to the memory folder. It intentionally does
//! not depend on the memory write pipeline.

pub mod citations;
mod metrics;
mod prompts;
pub mod usage;

use firefam_utils_absolute_path::AbsolutePathBuf;

pub use prompts::build_memory_tool_developer_instructions;

const MEMORY_TOOL_DEVELOPER_INSTRUCTIONS_SUMMARY_TOKEN_LIMIT: usize = 2_500;

pub fn memory_root(firefam_home: &AbsolutePathBuf) -> AbsolutePathBuf {
    firefam_home.join("memories")
}
