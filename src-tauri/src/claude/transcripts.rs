//! Historical (resumable) sessions from `~/.claude/projects/**/*.jsonl` and
//! `~/.claude/history.jsonl`, enriched with a title (first user prompt).
//!
//! Contract (implemented by the core agent):
//! - Read only the first 64 KB of a transcript to find `cwd`, `gitBranch`,
//!   `version` and the first real user prompt (skip lines whose text starts
//!   with `<`, tool results, and the "[Pasted text #N +M lines]" marker);
//!   collapse whitespace; truncate to 80 chars.
//! - Skip anything under a `subagents/` directory.
//! - Cache `(mtime, size) -> (title, cwd, branch, version)` as JSON in
//!   `cache_dir` (0600); never store message bodies.
//! - `recent_sessions` sorts by `last_active_at` desc; `status: Unknown`, `pid: None`.

use crate::model::Session;
use std::path::PathBuf;

pub struct TranscriptIndex {
    pub projects_dir: PathBuf,
    pub history_file: PathBuf,
    pub cache_dir: PathBuf,
}

impl TranscriptIndex {
    pub fn new(projects_dir: PathBuf, history_file: PathBuf, cache_dir: PathBuf) -> Self {
        Self { projects_dir, history_file, cache_dir }
    }

    /// Newest `limit` sessions known from disk.
    pub fn recent_sessions(&self, _limit: usize) -> Vec<Session> {
        // STUB — replaced by the core module implementation.
        Vec::new()
    }

    /// Fill `title`, `git_branch`, `version`, `transcript_path` (and `cwd` if empty).
    pub fn enrich(&self, session: Session) -> Session {
        // STUB
        session
    }

    /// Forget in-memory state so the next call rescans.
    pub fn invalidate(&self) {
        // STUB
    }
}
