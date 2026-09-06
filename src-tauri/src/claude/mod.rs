//! Read-only access to the data the Claude Code CLI writes under `~/.claude`.
//! See docs/ARCHITECTURE.md for the observed formats.

pub mod paths;
pub mod registry;
pub mod transcripts;
