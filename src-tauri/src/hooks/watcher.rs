//! Watches the spool directory for event files dropped by the hook script.
//!
//! Contract (implemented by the hooks agent):
//! - `notify` watcher on the directory plus a 3 s poll as backstop; process
//!   files in name order; delete each after parsing; delete unparseable or
//!   oversized (> 256 KB) files; create the dir (0700) if missing; process
//!   files that already exist at start.
//! - `parse` keeps only the fields in `HookEvent` (never `tool_input` /
//!   `user_input`); `session_id` is required; `message` truncated to 200 chars.

use crate::model::HookEvent;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing session_id")]
    MissingSessionId,
}

/// Handle that stops the watcher when dropped.
pub struct HookWatcher {
    _private: (),
}

impl HookWatcher {
    /// Start watching `events_dir`; `on_event` is called from a background thread.
    pub fn start<F>(_events_dir: PathBuf, _on_event: F) -> std::io::Result<Self>
    where
        F: Fn(HookEvent) + Send + 'static,
    {
        // STUB
        Ok(Self { _private: () })
    }

    pub fn stop(self) {}
}

/// Parse the raw stdin JSON that Claude Code handed the hook.
pub fn parse(_bytes: &[u8]) -> Result<HookEvent, ParseError> {
    // STUB
    Err(ParseError::MissingSessionId)
}
