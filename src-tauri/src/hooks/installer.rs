//! Registers Heron's hook receiver in `~/.claude/settings.json`.
//!
//! Contract (implemented by the hooks agent):
//! - Writes the receiver script to `script_path` (0700 on Unix). The script
//!   reads stdin (cap 256 KB), writes it atomically into `events_dir`
//!   (0700 dir, 0600 files, name `<ms>-<pid>.json`) and always exits 0 with
//!   no stdout. Unix: `#!/bin/sh` script. Windows: a PowerShell script.
//! - Registers it for `EVENTS` using the **exec form** (`command` + `args`,
//!   no shell): Unix `command = <script path>`; Windows
//!   `command = "powershell.exe"`, `args = ["-NoProfile","-NonInteractive",
//!   "-ExecutionPolicy","Bypass","-File", <script path>]`. Always
//!   `"async": true, "timeout": 5`. Notification matcher:
//!   `permission_prompt|idle_prompt|elicitation_dialog|elicitation_url_dialog|agent_needs_input|agent_completed`.
//! - Identifies its own groups by a hook whose command/args mention
//!   `heron-hook`; replaces those in place; never touches other groups;
//!   preserves unknown keys; timestamped backup (keep 5) before every write;
//!   atomic replace; preserves file mode.
//! - `uninstall` removes exactly Heron's groups (and empty arrays/keys).

use crate::model::{HookEventKind, HookStatus};
use std::path::PathBuf;

pub const EVENTS: [HookEventKind; 6] = [
    HookEventKind::SessionStart,
    HookEventKind::SessionEnd,
    HookEventKind::Stop,
    HookEventKind::Notification,
    HookEventKind::PermissionRequest,
    HookEventKind::UserPromptSubmit,
];

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("~/.claude/settings.json is not a JSON object; not touching it")]
    NotAnObject,
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

pub struct HookInstaller {
    pub settings_file: PathBuf,
    pub script_path: PathBuf,
    pub events_dir: PathBuf,
}

impl HookInstaller {
    pub fn new(settings_file: PathBuf, script_path: PathBuf, events_dir: PathBuf) -> Self {
        Self { settings_file, script_path, events_dir }
    }

    /// The receiver script for this platform.
    pub fn script_source(&self) -> String {
        // STUB
        String::new()
    }

    pub fn status(&self) -> HookStatus {
        // STUB
        HookStatus::NotInstalled
    }

    pub fn install(&self) -> Result<(), InstallError> {
        // STUB
        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), InstallError> {
        // STUB
        Ok(())
    }
}
