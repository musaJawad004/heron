//! Where Claude Code keeps its files on each platform.
//! `CLAUDE_CONFIG_DIR` relocates the whole tree; otherwise `~/.claude`
//! (`%USERPROFILE%\.claude` on Windows).

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ClaudePaths {
    pub root: PathBuf,
}

impl ClaudePaths {
    pub fn detect() -> Self {
        let root = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| home_dir().join(".claude"));
        Self { root }
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Live registry: one `<pid>.json` per running CLI. Never read `*.key` here.
    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }
    /// Transcripts grouped by encoded cwd.
    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }
    /// Prompt history index.
    pub fn history_file(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }
    /// User settings where hooks are registered.
    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Heron's own writable locations (under the OS app-data dir).
#[derive(Clone, Debug)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
    /// Hook scripts spool one JSON file per event here.
    pub fn events_dir(&self) -> PathBuf {
        self.data_dir.join("events")
    }
    /// Installed hook receiver script.
    pub fn hook_script(&self) -> PathBuf {
        let name = if cfg!(windows) { "heron-hook.ps1" } else { "heron-hook" };
        self.data_dir.join("bin").join(name)
    }
    /// Transcript title cache.
    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }
    /// One-shot launch scripts.
    pub fn launch_dir(&self) -> PathBuf {
        self.data_dir.join("launch")
    }
}
