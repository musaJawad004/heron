//! User preferences, persisted as JSON in the app data directory.
//! Nothing here ever leaves the machine.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::TerminalApp;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PanelEdge {
    #[default]
    Left,
    Right,
}

/// Saved panel position: physical pixels plus the monitor it was on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PanelOrigin {
    pub x: i32,
    pub y: i32,
    pub monitor: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // Launching
    /// `None` = auto-detect the best installed terminal.
    pub terminal_app: Option<TerminalApp>,
    /// Empty = auto-detect.
    pub claude_path: String,
    /// Extra CLI flags appended to every launch (whitespace separated).
    pub extra_claude_args: String,
    /// Folder the "New session" picker opens in. Empty = home.
    pub default_projects_folder: String,

    // Floating panel
    pub show_panel: bool,
    pub panel_edge: PanelEdge,
    pub panel_follows_active_screen: bool,
    pub panel_opacity: f64,
    pub panel_expanded: bool,
    pub panel_origin: Option<PanelOrigin>,

    // Notifications
    pub notify_on_permission: bool,
    pub notify_on_done: bool,
    pub notify_on_idle: bool,
    pub play_sound: bool,

    // General
    pub recent_limit: usize,
    pub show_count_in_tray: bool,
    pub launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            terminal_app: None,
            claude_path: String::new(),
            extra_claude_args: String::new(),
            default_projects_folder: String::new(),
            show_panel: true,
            panel_edge: PanelEdge::Left,
            panel_follows_active_screen: true,
            panel_opacity: 1.0,
            panel_expanded: true,
            panel_origin: None,
            notify_on_permission: true,
            notify_on_done: true,
            notify_on_idle: false,
            play_sound: true,
            recent_limit: 20,
            show_count_in_tray: true,
            launch_at_login: false,
        }
    }
}

impl Settings {
    pub fn extra_arg_list(&self) -> Vec<String> {
        self.extra_claude_args.split_whitespace().map(str::to_string).collect()
    }
}

/// Loads and saves `Settings` atomically.
pub struct SettingsStore {
    pub path: PathBuf,
}

impl SettingsStore {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("settings.json") }
    }

    pub fn load(&self) -> Settings {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                log::warn!("settings.json unreadable ({e}); using defaults");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, settings: &Settings) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
        fs::write(&tmp, json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
        }
        fs::rename(&tmp, &self.path)
    }
}
