//! macOS. Terminals are app bundles found by path (`/Applications`,
//! `~/Applications`, the system Utilities folder) with one cached Spotlight
//! query as a fallback. Everything is opened through `/usr/bin/open` with
//! the app's bundle id, so no shell is involved. Terminal.app and iTerm2 are
//! focused by tty through `osascript`; the tty is validated first and passed
//! as an argv item (`on run argv`), never spliced into the script. Nothing
//! here launches an app that is not already running when focusing.

use super::{run_with_timeout, spawn_detached, valid_tty, LaunchError};
use crate::model::{Session, TerminalApp};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

struct App {
    id: TerminalApp,
    /// Bundle name without `.app`, also the `Contents/MacOS` parent.
    bundle: &'static str,
    bundle_id: &'static str,
}

/// Preference order.
const APPS: &[App] = &[
    App { id: TerminalApp::Terminal, bundle: "Terminal", bundle_id: "com.apple.Terminal" },
    App { id: TerminalApp::Iterm, bundle: "iTerm", bundle_id: "com.googlecode.iterm2" },
    App { id: TerminalApp::Ghostty, bundle: "Ghostty", bundle_id: "com.mitchellh.ghostty" },
    App { id: TerminalApp::Warp, bundle: "Warp", bundle_id: "dev.warp.Warp-Stable" },
    App { id: TerminalApp::Kitty, bundle: "kitty", bundle_id: "net.kovidgoyal.kitty" },
    App { id: TerminalApp::Alacritty, bundle: "Alacritty", bundle_id: "org.alacritty" },
    App { id: TerminalApp::Wezterm, bundle: "WezTerm", bundle_id: "com.github.wez.wezterm" },
];

fn app(id: TerminalApp) -> Option<&'static App> {
    APPS.iter().find(|a| a.id == id)
}

fn bundle_paths(bundle: &str) -> Vec<PathBuf> {
    let name = format!("{bundle}.app");
    let mut paths = vec![
        PathBuf::from("/Applications").join(&name),
        PathBuf::from("/System/Applications/Utilities").join(&name),
        PathBuf::from("/Applications/Utilities").join(&name),
    ];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("Applications").join(&name));
    }
    paths
}

fn on_disk(app: &App) -> bool {
    bundle_paths(app.bundle).iter().any(|p| p.is_dir())
}

/// Bundles Spotlight knows about. One `mdfind` per process lifetime, capped
/// at 1 s; empty when Spotlight is unavailable or slow.
fn spotlight() -> &'static HashSet<TerminalApp> {
    static HITS: OnceLock<HashSet<TerminalApp>> = OnceLock::new();
    HITS.get_or_init(|| {
        let query = APPS
            .iter()
            .map(|a| format!("kMDItemCFBundleIdentifier == '{}'", a.bundle_id))
            .collect::<Vec<_>>()
            .join(" || ");
        let mut cmd = Command::new("/usr/bin/mdfind");
        cmd.args(["-attr", "kMDItemCFBundleIdentifier", &query]);
        let mut hits = HashSet::new();
        match run_with_timeout(&mut cmd, Duration::from_secs(1)) {
            Ok(Some(out)) if out.success() => {
                // `<path>   kMDItemCFBundleIdentifier = <id>`
                for line in out.stdout_text().lines() {
                    let Some((path, id)) = line.split_once("kMDItemCFBundleIdentifier = ") else { continue };
                    let path = path.trim();
                    if path.contains("/.Trash/") || !Path::new(path).is_dir() {
                        continue;
                    }
                    if let Some(a) = APPS.iter().find(|a| a.bundle_id == id.trim()) {
                        hits.insert(a.id);
                    }
                }
            }
            Ok(Some(out)) => log::debug!("mdfind failed: {}", out.stderr_text()),
            Ok(None) => log::debug!("mdfind did not answer within 1 s"),
            Err(e) => log::debug!("mdfind: {e}"),
        }
        hits
    })
}

pub fn installed_terminals() -> Vec<TerminalApp> {
    APPS.iter().filter(|a| on_disk(a) || spotlight().contains(&a.id)).map(|a| a.id).collect()
}

pub fn open_terminal(terminal: TerminalApp, cwd: &Path, script: &Path) -> Result<(), LaunchError> {
    let Some(app) = app(terminal) else { return Err(LaunchError::TerminalMissing(terminal)) };
    let mut cmd = Command::new("/usr/bin/open");
    match terminal {
        // These run a `.command` file in a new window themselves.
        TerminalApp::Terminal | TerminalApp::Iterm | TerminalApp::Warp => {
            cmd.args(["-b", app.bundle_id]).arg(script);
        }
        TerminalApp::Ghostty => {
            let mut wd = OsString::from("--working-directory=");
            wd.push(cwd.as_os_str());
            cmd.args(["-n", "-b", app.bundle_id, "--args"]).arg(wd).arg("-e").arg(script);
        }
        TerminalApp::Kitty => {
            cmd.args(["-n", "-b", app.bundle_id, "--args", "--directory"]).arg(cwd).arg(script);
        }
        TerminalApp::Alacritty => {
            cmd.args(["-n", "-b", app.bundle_id, "--args", "--working-directory"])
                .arg(cwd)
                .arg("-e")
                .arg(script);
        }
        TerminalApp::Wezterm => {
            cmd.args(["-n", "-b", app.bundle_id, "--args", "start", "--cwd"]).arg(cwd).arg("--").arg(script);
        }
        _ => return Err(LaunchError::TerminalMissing(terminal)),
    }
    spawn_detached(&mut cmd, Duration::from_secs(5), true)
}

/// Terminal.app: select the tab whose tty matches, raise its window.
const TERMINAL_SCRIPT: &str = r#"on run argv
    set target to "/dev/" & item 1 of argv
    tell application id "com.apple.Terminal"
        repeat with w in windows
            repeat with t in tabs of w
                if (tty of t) is target then
                    set selected of t to true
                    try
                        set miniaturized of w to false
                    end try
                    set index of w to 1
                    activate
                    return "focused"
                end if
            end repeat
        end repeat
    end tell
    return "notfound"
end run"#;

/// iTerm2: windows → tabs → sessions, then `select` each level.
const ITERM_SCRIPT: &str = r#"on run argv
    set target to "/dev/" & item 1 of argv
    tell application id "com.googlecode.iterm2"
        repeat with w in windows
            repeat with t in tabs of w
                repeat with s in sessions of t
                    if (tty of s) is target then
                        select s
                        select t
                        select w
                        activate
                        return "focused"
                    end if
                end repeat
            end repeat
        end repeat
    end tell
    return "notfound"
end run"#;

enum Outcome {
    Focused,
    NotFound,
    Failed,
}

fn focus_by_tty(terminal: TerminalApp, tty: &str) -> Outcome {
    let script = match terminal {
        TerminalApp::Terminal => TERMINAL_SCRIPT,
        TerminalApp::Iterm => ITERM_SCRIPT,
        _ => return Outcome::Failed,
    };
    debug_assert!(valid_tty(tty));
    let mut cmd = Command::new("/usr/bin/osascript");
    cmd.args(["-e", script, "--", tty]);
    match run_with_timeout(&mut cmd, Duration::from_secs(3)) {
        Ok(Some(out)) if out.success() => match out.stdout_text().as_str() {
            "focused" => Outcome::Focused,
            "notfound" => Outcome::NotFound,
            other => {
                log::debug!("osascript returned {other:?}");
                Outcome::Failed
            }
        },
        Ok(Some(out)) => {
            let err = out.stderr_text();
            if err.contains("-1743") {
                log::warn!(
                    "Automation permission for {} denied; allow Heron under System Settings > Privacy & Security > Automation",
                    terminal.display_name()
                );
            } else {
                log::warn!("osascript failed for {}: {err}", terminal.display_name());
            }
            Outcome::Failed
        }
        Ok(None) => {
            log::warn!("osascript did not answer within 3 s (an Automation prompt may be waiting)");
            Outcome::Failed
        }
        Err(e) => {
            log::warn!("osascript: {e}");
            Outcome::Failed
        }
    }
}

/// True when a process of that bundle is running (`pgrep` on the bundle's
/// `Contents/MacOS` path, so the process name does not matter).
fn is_running(terminal: TerminalApp) -> bool {
    let Some(app) = app(terminal) else { return false };
    let mut cmd = Command::new("/usr/bin/pgrep");
    cmd.args(["-f", "--", &format!("/{}.app/Contents/MacOS/", app.bundle)]);
    matches!(run_with_timeout(&mut cmd, Duration::from_secs(2)), Ok(Some(out)) if out.success())
}

/// Bring the app itself forward (no tab selection possible).
fn activate(terminal: TerminalApp) {
    let Some(app) = app(terminal) else { return };
    let mut cmd = Command::new("/usr/bin/open");
    cmd.args(["-b", app.bundle_id]);
    if let Err(e) = spawn_detached(&mut cmd, Duration::from_secs(3), true) {
        log::debug!("activate {}: {e}", terminal.display_name());
    }
}

pub fn focus(session: &Session, terminal: TerminalApp) -> bool {
    let Some(tty) = session.tty.as_deref().filter(|t| valid_tty(t)) else {
        log::debug!("focus: session {} has no usable tty", session.id);
        return false;
    };
    let installed = installed_terminals();
    // The chosen terminal first, then the other scriptable one: a session
    // started by hand may live in either.
    let scriptable = match terminal {
        TerminalApp::Iterm => [TerminalApp::Iterm, TerminalApp::Terminal],
        _ => [TerminalApp::Terminal, TerminalApp::Iterm],
    };
    for app in scriptable.into_iter().filter(|a| installed.contains(a) && is_running(*a)) {
        match focus_by_tty(app, tty) {
            Outcome::Focused => return true,
            Outcome::NotFound => log::debug!("{tty} is not a tab of {}", app.display_name()),
            Outcome::Failed => {}
        }
    }
    let scriptable_choice = matches!(terminal, TerminalApp::Terminal | TerminalApp::Iterm);
    if !scriptable_choice && installed.contains(&terminal) && is_running(terminal) {
        activate(terminal);
    }
    false
}

pub fn terminate(pid: u32) -> Result<(), LaunchError> {
    super::kill_term(pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn terminal_app_is_always_found() {
        let list = installed_terminals();
        assert_eq!(list.first(), Some(&TerminalApp::Terminal), "{list:?}");
        assert!(on_disk(app(TerminalApp::Terminal).unwrap()));
    }

    #[test]
    fn spotlight_is_cached_and_never_returns_foreign_apps() {
        let a = spotlight() as *const _;
        let b = spotlight() as *const _;
        assert_eq!(a, b);
        assert!(spotlight().iter().all(|t| app(*t).is_some()));
    }

    #[test]
    fn focus_without_tty_is_false_without_scripting() {
        let s = Session { pid: Some(4242), tty: None, ..Default::default() };
        assert!(!focus(&s, TerminalApp::Terminal));
        let s = Session { pid: Some(4242), tty: Some("pts/3".into()), ..Default::default() };
        assert!(!focus(&s, TerminalApp::Terminal));
    }

    #[test]
    fn open_rejects_non_mac_terminals() {
        let err =
            open_terminal(TerminalApp::Cmd, Path::new("/tmp"), Path::new("/tmp/x.command")).unwrap_err();
        assert!(matches!(err, LaunchError::TerminalMissing(TerminalApp::Cmd)));
    }

    fn count_terminal_windows() -> i64 {
        let mut cmd = Command::new("/usr/bin/osascript");
        cmd.args(["-e", r#"tell application id "com.apple.Terminal" to count windows"#]);
        run_with_timeout(&mut cmd, Duration::from_secs(5))
            .ok()
            .flatten()
            .and_then(|o| o.stdout_text().parse().ok())
            .unwrap_or(-1)
    }

    fn terminal_window_names() -> Vec<String> {
        let mut cmd = Command::new("/usr/bin/osascript");
        cmd.args(["-e", r#"tell application id "com.apple.Terminal" to get name of every window"#]);
        run_with_timeout(&mut cmd, Duration::from_secs(5))
            .ok()
            .flatten()
            .map(|o| o.stdout_text().split(", ").map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// `cargo test live_launch -- --ignored --nocapture`: opens a real
    /// Terminal.app window running `/bin/echo heron-launch-test` (never the
    /// real `claude`), checks a window appeared, then closes it.
    #[test]
    #[ignore]
    fn live_launch() {
        let tmp = tempfile::tempdir().unwrap();
        let before = count_terminal_windows();
        let before_names = terminal_window_names();
        eprintln!("Terminal windows before: {before}");
        super::super::launch(super::super::LaunchRequest {
            cwd: Path::new("/tmp"),
            terminal: TerminalApp::Terminal,
            claude: Path::new("/bin/echo"),
            args: vec!["heron-launch-test".into()],
            launch_dir: tmp.path(),
        })
        .expect("launch");
        let mut after = before;
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(250));
            after = count_terminal_windows();
            if after > before {
                break;
            }
        }
        let after_names = terminal_window_names();
        let new_names: Vec<_> = after_names.iter().filter(|n| !before_names.contains(n)).cloned().collect();
        eprintln!("Terminal windows after: {after}; new windows: {new_names:?}");
        // The script removes itself as its first action.
        std::thread::sleep(Duration::from_secs(1));
        let leftovers: Vec<PathBuf> =
            std::fs::read_dir(tmp.path()).unwrap().flatten().map(|e| e.path()).collect();
        eprintln!("scripts left in launch dir: {leftovers:?}");
        // Best-effort cleanup of the window we opened.
        let mut cmd = Command::new("/usr/bin/osascript");
        cmd.args([
            "-e",
            r#"tell application id "com.apple.Terminal" to close (every window whose name contains "heron-launch-test" or name contains "heron-")"#,
        ]);
        let _ = run_with_timeout(&mut cmd, Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(500));
        eprintln!("Terminal windows after cleanup: {}", count_terminal_windows());
        assert!(after > before, "no new Terminal window appeared");
        assert!(leftovers.is_empty(), "launch script did not delete itself");
    }

    /// `cargo test live_focus -- --ignored --nocapture`: takes a running
    /// session from `~/.claude/sessions/*.json` (never `.key`), resolves its
    /// tty with `ps`, and asks Terminal.app to focus it. The first run
    /// triggers the macOS Automation prompt.
    #[test]
    #[ignore]
    fn live_focus() {
        let root = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap().join(".claude"));
        let Ok(entries) = std::fs::read_dir(root.join("sessions")) else {
            eprintln!("no sessions dir; nothing to focus");
            return;
        };
        let mut candidates = Vec::new();
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
            let Some(pid) = v.get("pid").and_then(|p| p.as_u64()) else { continue };
            let id = v.get("sessionId").and_then(|s| s.as_str()).unwrap_or("").to_string();
            let cwd = v.get("cwd").and_then(|s| s.as_str()).unwrap_or("").to_string();
            candidates.push((pid as u32, id, cwd));
        }
        if candidates.is_empty() {
            eprintln!("no running sessions; nothing to focus");
            return;
        }
        for (pid, id, cwd) in candidates {
            let mut ps = Command::new("/bin/ps");
            ps.args(["-o", "tty=", "-p", &pid.to_string()]);
            let tty =
                run_with_timeout(&mut ps, Duration::from_secs(5)).ok().flatten().map(|o| o.stdout_text());
            let tty = tty.filter(|t| valid_tty(t));
            eprintln!("session pid {pid} ({id}) in {cwd}: tty {tty:?}");
            let session = Session { id, cwd, pid: Some(pid), tty, ..Default::default() };
            let started = std::time::Instant::now();
            let ok = focus(&session, TerminalApp::Terminal);
            eprintln!("focus -> {ok} in {:?}", started.elapsed());
            if ok {
                return;
            }
        }
        eprintln!("no session could be focused (see log lines above: Automation prompt / other terminal)");
    }
}
