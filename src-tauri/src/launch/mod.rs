//! Opens Claude Code sessions in the user's terminal and raises existing ones.
//!
//! Contract (implemented by the launch agent):
//! - `find_claude`: PATH, then `~/.local/bin/claude[.exe]`, Homebrew, npm
//!   global, volta, bun. GUI apps get a minimal PATH, hence the fallbacks.
//! - `new_session` runs `claude [extra]` in `cwd`; `resume` runs
//!   `claude --resume <id> [extra]` in the session cwd (error if missing).
//! - Launch via a one-shot script in `launch_dir` that self-deletes and
//!   `exec`s claude (Unix) / a `.cmd` (Windows), opened with the terminal's
//!   own CLI or file association; never a shell string built by
//!   concatenation — use `shell_quote` for every value inside the script.
//! - `focus`: macOS Terminal/iTerm2 by tty via `osascript` (validate the tty
//!   against `^ttys?[0-9]+$` first); Windows by finding the console window
//!   owning the pid tree; Linux best effort. Returns false when it cannot.

use crate::model::{Session, TerminalApp, TerminalChoice};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{} is not installed", .0.display_name())]
    TerminalMissing(TerminalApp),
    #[error("the folder no longer exists: {0}")]
    FolderMissing(String),
    #[error("could not find the claude executable; set its path in Settings")]
    ClaudeMissing,
    #[error("{0}")]
    Other(String),
}

/// POSIX single-quote quoting. `'` becomes `'\''`.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Terminals installed on this machine, in preference order.
pub fn installed_terminals() -> Vec<TerminalChoice> {
    // STUB
    let default = if cfg!(target_os = "macos") {
        TerminalApp::Terminal
    } else if cfg!(windows) {
        TerminalApp::Cmd
    } else {
        TerminalApp::Xterm
    };
    vec![TerminalChoice { id: default, name: default.display_name() }]
}

/// The terminal to use when the user has not chosen one.
pub fn default_terminal() -> TerminalApp {
    installed_terminals().first().map(|t| t.id).unwrap_or(TerminalApp::Terminal)
}

/// Locate the `claude` executable.
pub fn find_claude() -> Option<PathBuf> {
    // STUB
    None
}

pub struct LaunchRequest<'a> {
    pub cwd: &'a Path,
    pub terminal: TerminalApp,
    pub claude: &'a Path,
    pub args: Vec<String>,
    pub launch_dir: &'a Path,
}

pub fn launch(_req: LaunchRequest<'_>) -> Result<(), LaunchError> {
    // STUB
    Ok(())
}

/// Bring the terminal tab/window of a running session to the front.
pub fn focus(_session: &Session, _terminal: TerminalApp) -> bool {
    // STUB
    false
}

/// Ask the OS to end a running session politely (SIGTERM / taskkill).
pub fn terminate(_pid: u32) -> Result<(), LaunchError> {
    // STUB
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
