//! The apply engine: turning a profile into a working system.
//!
//! The steps are always the same:
//! 1. **plan**: work out what is missing (packages, setup steps) and what
//!    was dropped from the profile, without changing anything;
//! 2. **confirm**: show the plan and ask;
//! 3. **execute**: run the commands, one step at a time, logging everything;
//! 4. **report**: say honestly what worked, failed or was skipped.

pub mod checks;
pub mod command;
pub mod engine;
pub mod log;
pub mod plan;
pub mod prompt;
pub mod state;
