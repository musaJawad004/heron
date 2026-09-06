//! Linux and other Unix. Terminals are executables on `PATH`, each opened
//! with its own working-directory and command flags (never via a shell).
//! `x-terminal-emulator` (Debian alternatives) backs the `Xterm` choice
//! when `xterm` itself is absent. Focus is best effort on X11: walk the
//! session's ancestors through `/proc` (the terminal or its server is one of
//! them) and activate a window owned by one of those pids with `wmctrl`
//! or `xdotool`; returns false on Wayland or without those tools.

use super::{find_on_path, run_with_timeout, spawn_detached, LaunchError};
use crate::model::{Session, TerminalApp};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Preference order before the desktop adjustment.
const TERMS: &[(TerminalApp, &str)] = &[
    (TerminalApp::GnomeTerminal, "gnome-terminal"),
    (TerminalApp::Konsole, "konsole"),
    (TerminalApp::Kitty, "kitty"),
    (TerminalApp::Alacritty, "alacritty"),
    (TerminalApp::Wezterm, "wezterm"),
    (TerminalApp::Ghostty, "ghostty"),
    (TerminalApp::Xterm, "xterm"),
];

fn binary(id: TerminalApp) -> Option<PathBuf> {
    let (_, bin) = TERMS.iter().find(|(t, _)| *t == id)?;
    let found = find_on_path(bin);
    if found.is_none() && id == TerminalApp::Xterm {
        return find_on_path("x-terminal-emulator");
    }
    found
}

pub fn installed_terminals() -> Vec<TerminalApp> {
    let mut list: Vec<TerminalApp> = TERMS.iter().map(|(t, _)| *t).filter(|t| binary(*t).is_some()).collect();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_ascii_uppercase();
    let preferred = if desktop.contains("KDE") {
        Some(TerminalApp::Konsole)
    } else if desktop.contains("GNOME") {
        Some(TerminalApp::GnomeTerminal)
    } else {
        None
    };
    if let Some(pos) = preferred.and_then(|p| list.iter().position(|t| *t == p)) {
        let t = list.remove(pos);
        list.insert(0, t);
    }
    list
}

fn with_working_directory(cwd: &Path) -> OsString {
    let mut arg = OsString::from("--working-directory=");
    arg.push(cwd.as_os_str());
    arg
}

pub fn open_terminal(terminal: TerminalApp, cwd: &Path, script: &Path) -> Result<(), LaunchError> {
    let bin = binary(terminal).ok_or(LaunchError::TerminalMissing(terminal))?;
    let mut cmd = Command::new(&bin);
    match terminal {
        TerminalApp::GnomeTerminal => {
            cmd.arg(with_working_directory(cwd)).arg("--").arg(script);
        }
        TerminalApp::Konsole => {
            cmd.arg("--workdir").arg(cwd).arg("-e").arg(script);
        }
        TerminalApp::Kitty => {
            cmd.arg("--directory").arg(cwd).arg(script);
        }
        TerminalApp::Alacritty => {
            cmd.arg("--working-directory").arg(cwd).arg("-e").arg(script);
        }
        TerminalApp::Wezterm => {
            cmd.arg("start").arg("--cwd").arg(cwd).arg("--").arg(script);
        }
        TerminalApp::Ghostty => {
            cmd.arg(with_working_directory(cwd)).arg("-e").arg(script);
        }
        // Both `xterm` and `x-terminal-emulator` take `-e <command>`.
        TerminalApp::Xterm => {
            cmd.arg("-e").arg(script);
        }
        _ => return Err(LaunchError::TerminalMissing(terminal)),
    }
    cmd.current_dir(cwd);
    spawn_detached(&mut cmd, Duration::from_millis(500), true)
}

/// `/proc/<pid>/stat`: `pid (comm) state ppid ...`; comm may contain spaces.
fn parent_pid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = &stat[stat.rfind(')')? + 1..];
    let mut fields = rest.split_whitespace();
    fields.next()?; // state
    fields.next()?.parse().ok()
}

/// The pid and its ancestors, closest first, stopping before init and Heron.
fn ancestors(pid: u32) -> Vec<u32> {
    let me = std::process::id();
    let mut chain = Vec::new();
    let mut cur = pid;
    for _ in 0..12 {
        if cur <= 1 || cur == me || chain.contains(&cur) {
            break;
        }
        chain.push(cur);
        let Some(parent) = parent_pid(cur) else { break };
        cur = parent;
    }
    chain
}

/// `wmctrl -lp` lines: `0x03400003  0 12345  host  title`.
fn focus_wmctrl(bin: &Path, chain: &[u32]) -> bool {
    let mut list = Command::new(bin);
    list.arg("-lp");
    let Ok(Some(out)) = run_with_timeout(&mut list, Duration::from_secs(2)) else { return false };
    if !out.success() {
        return false;
    }
    let text = out.stdout_text();
    let windows: Vec<(&str, u32)> = text
        .lines()
        .filter_map(|line| {
            let mut f = line.split_whitespace();
            let wid = f.next()?;
            f.next()?; // desktop
            let pid = f.next()?.parse().ok()?;
            Some((wid, pid))
        })
        .collect();
    for pid in chain {
        if let Some((wid, _)) = windows.iter().find(|(_, p)| p == pid) {
            let mut raise = Command::new(bin);
            raise.args(["-ia", wid]);
            return matches!(run_with_timeout(&mut raise, Duration::from_secs(2)), Ok(Some(o)) if o.success());
        }
    }
    false
}

fn focus_xdotool(bin: &Path, chain: &[u32]) -> bool {
    for pid in chain {
        let mut search = Command::new(bin);
        search.args(["search", "--onlyvisible", "--pid", &pid.to_string()]);
        let Ok(Some(out)) = run_with_timeout(&mut search, Duration::from_secs(2)) else { continue };
        let text = out.stdout_text();
        let Some(wid) = text.lines().next().map(str::trim).filter(|w| w.bytes().all(|b| b.is_ascii_digit()))
        else {
            continue;
        };
        let mut raise = Command::new(bin);
        raise.args(["windowactivate", "--sync", wid]);
        if matches!(run_with_timeout(&mut raise, Duration::from_secs(2)), Ok(Some(o)) if o.success()) {
            return true;
        }
    }
    false
}

pub fn focus(session: &Session, _terminal: TerminalApp) -> bool {
    let Some(pid) = session.pid else { return false };
    let chain = ancestors(pid);
    if chain.is_empty() {
        return false;
    }
    if let Some(wmctrl) = find_on_path("wmctrl") {
        if focus_wmctrl(&wmctrl, &chain) {
            return true;
        }
    }
    if let Some(xdotool) = find_on_path("xdotool") {
        if focus_xdotool(&xdotool, &chain) {
            return true;
        }
    }
    log::debug!("focus: no window found for pid {pid} (Wayland, or wmctrl/xdotool missing)");
    false
}

pub fn terminate(pid: u32) -> Result<(), LaunchError> {
    super::kill_term(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancestors_start_with_the_pid_and_exclude_heron() {
        let me = std::process::id();
        assert!(ancestors(me).is_empty());
        let chain = ancestors(4_000_000);
        assert_eq!(chain, vec![4_000_000]); // no such process: just itself
        if let Some(parent) = parent_pid(me) {
            assert!(parent >= 1);
        }
    }

    #[test]
    fn open_rejects_non_linux_terminals() {
        let err =
            open_terminal(TerminalApp::Terminal, Path::new("/tmp"), Path::new("/tmp/x.sh")).unwrap_err();
        assert!(matches!(err, LaunchError::TerminalMissing(TerminalApp::Terminal)));
    }
}
