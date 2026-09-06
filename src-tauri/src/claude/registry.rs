//! Live sessions from `~/.claude/sessions/<pid>.json`.
//!
//! Contract (implemented by the core agent):
//! - `snapshot()` parses every `*.json` (never `*.key`), maps fields as in
//!   docs/ARCHITECTURE.md (ms timestamps), and drops entries whose pid is not
//!   alive or whose process is not a `claude` process (pid reuse). Liveness
//!   and command-line lookups are cached per pid for ~30 s.
//! - Fills `Session.tty` on Unix from the process table.
//! - Must be cheap (a few small files) and never panic on malformed input.

use crate::model::Session;
use std::path::PathBuf;

pub struct Registry {
    pub dir: PathBuf,
}

impl Registry {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// All live sessions, newest `started_at` first.
    pub fn snapshot(&self) -> Vec<Session> {
        // STUB — replaced by the core module implementation.
        Vec::new()
    }
}

/// True if a process with this pid exists (any owner).
pub fn is_pid_alive(_pid: u32) -> bool {
    // STUB
    false
}
