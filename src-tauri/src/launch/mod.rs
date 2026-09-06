//! Opens Claude Code sessions in the user's terminal and raises existing ones.
//!
//! Contract:
//! - `find_claude`: PATH, then `~/.local/bin/claude[.exe]`, Homebrew, npm
//!   global, volta, bun, nvm (newest version that has it). GUI apps get a
//!   minimal PATH, hence the fallbacks.
//! - `launch` runs `claude [args]` in `cwd` (`state.rs` passes
//!   `--resume <id>` in `args` for a resume; the folder must exist).
//! - Launch via a one-shot script in `launch_dir` that self-deletes and
//!   `exec`s claude (Unix) / a `.cmd` (Windows), opened with the terminal's
//!   own CLI or file association; never a shell string built by
//!   concatenation — every value inside a script goes through `shell_quote`
//!   (POSIX) or `powershell_quote`; Windows batch values are validated
//!   instead of escaped (see `script.rs`).
//! - `focus`: macOS Terminal/iTerm2 by tty via `osascript` (validate the tty
//!   against `^ttys?[0-9]+$` first); Windows by activating a window of the
//!   pid tree; Linux best effort with `wmctrl`/`xdotool`. Returns false when
//!   it cannot, and never launches an application that is not running.
//! - Processes are spawned with `std::process::Command` and argument arrays,
//!   never through `sh -c`. Helpers that wait for output are time-boxed so a
//!   stuck child cannot hang a command.
//!
//! Platform code lives in `macos.rs`, `windows.rs`, `linux.rs` (cfg-gated);
//! `script.rs` renders the launch scripts and is tested on every OS.

use crate::model::{Session, TerminalApp, TerminalChoice};
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

mod script;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as os;

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(all(unix, not(target_os = "macos")))]
use linux as os;

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

/// POSIX single-quote quoting. `'` becomes `'\''`. Safe for any string
/// (newlines, `$`, backticks and unicode are inert inside single quotes).
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// PowerShell single-quote quoting. `'` becomes `''`; nothing else is
/// interpreted inside a single-quoted PowerShell string.
pub fn powershell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Terminals installed on this machine, in preference order.
pub fn installed_terminals() -> Vec<TerminalChoice> {
    os::installed_terminals().into_iter().map(|id| TerminalChoice { id, name: id.display_name() }).collect()
}

/// The terminal to use when the user has not chosen one.
pub fn default_terminal() -> TerminalApp {
    let fallback = if cfg!(target_os = "macos") {
        TerminalApp::Terminal
    } else if cfg!(windows) {
        TerminalApp::Cmd
    } else {
        TerminalApp::Xterm
    };
    installed_terminals().first().map(|t| t.id).unwrap_or(fallback)
}

/// Locate the `claude` executable.
pub fn find_claude() -> Option<PathBuf> {
    find_claude_in(std::env::var_os("PATH").as_deref(), dirs::home_dir().as_deref(), CLAUDE_NAMES)
}

#[cfg(windows)]
const CLAUDE_NAMES: &[&str] = &["claude.exe", "claude.cmd"];
#[cfg(not(windows))]
const CLAUDE_NAMES: &[&str] = &["claude"];

/// The search itself, parameterised so tests can point it at a fixture home.
/// `PATH` entries first, then the well-known install locations.
fn find_claude_in(path_var: Option<&OsStr>, home: Option<&Path>, names: &[&str]) -> Option<PathBuf> {
    if let Some(path_var) = path_var {
        for dir in std::env::split_paths(path_var).filter(|d| !d.as_os_str().is_empty()) {
            if let Some(hit) = names.iter().map(|n| dir.join(n)).find(|p| is_executable(p)) {
                return Some(hit);
            }
        }
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = home {
        dirs.push(home.join(".local").join("bin"));
    }
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    if let Some(home) = home {
        dirs.push(home.join(".npm-global").join("bin"));
        dirs.push(home.join(".volta").join("bin"));
        dirs.push(home.join(".bun").join("bin"));
        dirs.extend(nvm_bin_dirs(&home.join(".nvm").join("versions").join("node")));
        // Older `claude migrate-installer` layout; an existence check only.
        dirs.push(home.join(".claude").join("local"));
    }
    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }
    dirs.iter().flat_map(|d| names.iter().map(move |n| d.join(n))).find(|p| is_executable(p))
}

/// `<nvm>/v22.1.0/bin`, newest version first, so the first executable wins.
fn nvm_bin_dirs(versions: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(versions) else { return Vec::new() };
    let mut found: Vec<(Vec<u64>, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| (version_key(&e.file_name().to_string_lossy()), e.path().join("bin")))
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, p)| p).collect()
}

/// `v22.1.0` → `[22, 1, 0]`; unparsable parts sort lowest.
fn version_key(name: &str) -> Vec<u64> {
    name.trim_start_matches('v').split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect()
}

/// A regular file (through symlinks) that the owner can execute.
pub(crate) fn is_executable(p: &Path) -> bool {
    let Ok(md) = std::fs::metadata(p) else { return false };
    if !md.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        md.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// First executable named `name` on `PATH`.
#[allow(dead_code)] // used by the Windows and Linux modules
pub(crate) fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var)
        .filter(|d| !d.as_os_str().is_empty())
        .map(|d| d.join(name))
        .find(|p| is_executable(p))
}

/// `ttys003`, `tty1` — the only shapes `focus` will ever hand to a script.
#[allow(dead_code)] // used by the macOS module
pub(crate) fn valid_tty(tty: &str) -> bool {
    let Some(rest) = tty.strip_prefix("tty") else { return false };
    let digits = rest.strip_prefix('s').unwrap_or(rest);
    !digits.is_empty() && digits.len() <= 8 && digits.bytes().all(|b| b.is_ascii_digit())
}

pub struct LaunchRequest<'a> {
    pub cwd: &'a Path,
    pub terminal: TerminalApp,
    pub claude: &'a Path,
    pub args: Vec<String>,
    pub launch_dir: &'a Path,
}

/// Write a one-shot launch script and open it in the requested terminal.
pub fn launch(req: LaunchRequest<'_>) -> Result<(), LaunchError> {
    if !req.cwd.is_dir() {
        return Err(LaunchError::FolderMissing(req.cwd.display().to_string()));
    }
    // A bare name (`claude`) is resolved by the script's PATH; a path must exist.
    let has_dir = req.claude.parent().is_some_and(|p| !p.as_os_str().is_empty());
    if has_dir && !req.claude.is_file() {
        return Err(LaunchError::ClaudeMissing);
    }
    if !os::installed_terminals().contains(&req.terminal) {
        return Err(LaunchError::TerminalMissing(req.terminal));
    }
    script::sweep_stale(req.launch_dir);
    let spec = script::Spec { cwd: req.cwd, exe: req.claude, args: &req.args };
    let path = script::write(req.launch_dir, &spec, script::Flavor::current())?;
    match os::open_terminal(req.terminal, req.cwd, &path) {
        Ok(()) => {
            log::info!("opened a session in {} at {}", req.terminal.display_name(), req.cwd.display());
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Err(e)
        }
    }
}

/// Bring the terminal tab/window of a running session to the front.
pub fn focus(session: &Session, terminal: TerminalApp) -> bool {
    os::focus(session, terminal)
}

/// Ask the OS to end a running session politely (SIGTERM / taskkill).
pub fn terminate(pid: u32) -> Result<(), LaunchError> {
    if pid <= 1 {
        return Err(LaunchError::Other(format!("refusing to signal pid {pid}")));
    }
    os::terminate(pid)
}

/// `kill -TERM <pid>` through the `kill` binary (no libc crate). Shared by
/// the macOS and Linux modules.
#[cfg(unix)]
pub(crate) fn kill_term(pid: u32) -> Result<(), LaunchError> {
    let pid_s = pid.to_string();
    let run = |program: &str| {
        let mut cmd = Command::new(program);
        cmd.args(["-TERM", &pid_s]);
        run_with_timeout(&mut cmd, Duration::from_secs(5))
    };
    let out = match run("/bin/kill") {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => run("kill")?,
        other => other?,
    };
    match out {
        Some(o) if o.success() => Ok(()),
        Some(o) => {
            let err = o.stderr_text();
            Err(LaunchError::Other(if err.is_empty() {
                format!("could not signal process {pid}")
            } else {
                err
            }))
        }
        None => Err(LaunchError::Other("kill did not finish".into())),
    }
}

// ---- process helpers shared by the platform modules --------------------------

/// Output of a child that ran to completion.
pub(crate) struct Finished {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Finished {
    pub fn success(&self) -> bool {
        self.status.success()
    }
    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }
}

fn drain<R: Read + Send + 'static>(reader: Option<R>) -> Option<JoinHandle<Vec<u8>>> {
    let mut reader = reader?;
    Some(thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    }))
}

/// Run `cmd` to completion capturing its output. Returns `Ok(None)` when it
/// did not finish within `timeout` (the child is killed).
pub(crate) fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> std::io::Result<Option<Finished>> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(15));
    };
    let stdout = out.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = err.and_then(|h| h.join().ok()).unwrap_or_default();
    Ok(status.map(|status| Finished { status, stdout, stderr }))
}

/// Spawn a child that is expected to keep running (a terminal). Waits up to
/// `grace` to catch an immediate failure (non-zero exit → error, with its
/// stderr when `capture_stderr`), then hands the child to a reaper thread so
/// it never becomes a zombie and its stderr pipe stays drained. Pass
/// `capture_stderr = false` for console programs that must own a fresh
/// console (Windows): redirecting their stdio would hide their UI.
pub(crate) fn spawn_detached(
    cmd: &mut Command,
    grace: Duration,
    capture_stderr: bool,
) -> Result<(), LaunchError> {
    let name = Path::new(cmd.get_program()).file_name().map(|s| s.to_string_lossy().into_owned());
    let name = name.unwrap_or_else(|| "process".into());
    if capture_stderr {
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    }
    let mut child = cmd.spawn()?;
    let tail: Arc<Mutex<Vec<u8>>> = Arc::default();
    if let Some(mut stderr) = child.stderr.take() {
        let tail = Arc::clone(&tail);
        thread::spawn(move || {
            let mut chunk = [0u8; 1024];
            while let Ok(n) = stderr.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                if let Ok(mut t) = tail.lock() {
                    if t.len() < 4096 {
                        t.extend_from_slice(&chunk[..n]);
                    }
                }
            }
        });
    }
    let deadline = Instant::now() + grace;
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
            let msg = tail.lock().map(|t| String::from_utf8_lossy(&t).trim().to_string()).unwrap_or_default();
            let msg = if msg.is_empty() {
                format!("{name} exited with {status}")
            } else {
                format!("{name}: {msg}")
            };
            return Err(LaunchError::Other(msg));
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let _ = thread::Builder::new().name("heron-reap".into()).spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn quoting() {
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote(""), "''");
        assert_eq!(shell_quote("$HOME"), "'$HOME'");
        assert_eq!(shell_quote("`id`"), "'`id`'");
        assert_eq!(shell_quote("a\nb"), "'a\nb'");
        assert_eq!(shell_quote("say \"hi\""), "'say \"hi\"'");
        assert_eq!(shell_quote("'; rm -rf / #"), "''\\''; rm -rf / #'");
        assert_eq!(shell_quote("héron/日本"), "'héron/日本'");
        assert_eq!(shell_quote("''"), "''\\'''\\'''");
    }

    #[test]
    fn powershell_quoting() {
        assert_eq!(powershell_quote("a b"), "'a b'");
        assert_eq!(powershell_quote("it's"), "'it''s'");
        assert_eq!(powershell_quote("$env:X `n \"q\""), "'$env:X `n \"q\"'");
        assert_eq!(powershell_quote(""), "''");
    }

    #[test]
    fn tty_validation() {
        for ok in ["ttys000", "ttys003", "tty1", "ttys12345"] {
            assert!(valid_tty(ok), "{ok}");
        }
        for bad in
            ["", "tty", "ttys", "pts/3", "ttys003; rm", "/dev/ttys003", "TTYS003", "ttys00a", "ttyss1", "??"]
        {
            assert!(!valid_tty(bad), "{bad}");
        }
    }

    #[test]
    fn version_keys_order_semantically() {
        assert!(version_key("v22.10.0") > version_key("v22.9.1"));
        assert!(version_key("v10.0.0") > version_key("v9.99.99"));
        assert_eq!(version_key("junk"), vec![0]);
    }

    fn make_exe(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn find_claude_prefers_path_then_local_bin() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let names = &["claude"];
        assert_eq!(find_claude_in(None, Some(&home), names), None);

        let local = home.join(".local/bin/claude");
        make_exe(&local);
        assert_eq!(find_claude_in(None, Some(&home), names), Some(local.clone()));

        let on_path = tmp.path().join("bin/claude");
        make_exe(&on_path);
        let path_var = std::env::join_paths([tmp.path().join("bin"), tmp.path().join("nope")]).unwrap();
        assert_eq!(find_claude_in(Some(&path_var), Some(&home), names), Some(on_path));

        // An empty PATH entry is ignored, a missing directory is harmless.
        let path_var = std::env::join_paths([PathBuf::new(), tmp.path().join("missing")]).unwrap();
        assert_eq!(find_claude_in(Some(&path_var), Some(&home), names), Some(local));
    }

    #[cfg(unix)]
    #[test]
    fn find_claude_requires_executable_bit() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let local = home.join(".local/bin/claude");
        make_exe(&local);
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&local, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(find_claude_in(None, Some(&home), &["claude"]), None);
        // A directory named claude never counts.
        let dir = home.join(".volta/bin/claude");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(find_claude_in(None, Some(&home), &["claude"]), None);
    }

    #[test]
    fn find_claude_picks_newest_nvm_that_has_it() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let nvm = home.join(".nvm/versions/node");
        make_exe(&nvm.join("v9.11.2/bin/claude"));
        make_exe(&nvm.join("v20.3.0/bin/claude"));
        fs::create_dir_all(nvm.join("v22.0.0/bin")).unwrap(); // newest, but no claude
        assert_eq!(find_claude_in(None, Some(&home), &["claude"]), Some(nvm.join("v20.3.0/bin/claude")));
    }

    #[test]
    fn find_claude_on_this_machine() {
        // Documentation of the real fallback: with no PATH at all, the user's
        // own `~/.local/bin/claude` is still found. Skipped where absent.
        let Some(home) = dirs::home_dir() else { return };
        let local = home.join(".local").join("bin").join(CLAUDE_NAMES[0]);
        if !is_executable(&local) {
            eprintln!("skipping: {} not present", local.display());
            return;
        }
        assert_eq!(find_claude_in(None, Some(&home), CLAUDE_NAMES), Some(local));
        assert!(find_claude().is_some());
    }

    #[test]
    fn launch_rejects_missing_folder_before_touching_anything() {
        let tmp = tempfile::tempdir().unwrap();
        let launch_dir = tmp.path().join("launch");
        let err = launch(LaunchRequest {
            cwd: &tmp.path().join("gone"),
            terminal: default_terminal(),
            claude: Path::new("claude"),
            args: vec![],
            launch_dir: &launch_dir,
        })
        .unwrap_err();
        assert!(matches!(err, LaunchError::FolderMissing(_)), "{err}");
        assert!(!launch_dir.exists());
    }

    #[test]
    fn launch_rejects_missing_claude_path() {
        let tmp = tempfile::tempdir().unwrap();
        let err = launch(LaunchRequest {
            cwd: tmp.path(),
            terminal: default_terminal(),
            claude: &tmp.path().join("no-such-claude"),
            args: vec![],
            launch_dir: &tmp.path().join("launch"),
        })
        .unwrap_err();
        assert!(matches!(err, LaunchError::ClaudeMissing), "{err}");
    }

    #[test]
    fn launch_rejects_uninstalled_terminal() {
        let tmp = tempfile::tempdir().unwrap();
        // GNOME Terminal is never reported on macOS/Windows; Warp never on Linux.
        let foreign = if cfg!(target_os = "linux") { TerminalApp::Warp } else { TerminalApp::GnomeTerminal };
        let err = launch(LaunchRequest {
            cwd: tmp.path(),
            terminal: foreign,
            claude: Path::new("claude"),
            args: vec![],
            launch_dir: &tmp.path().join("launch"),
        })
        .unwrap_err();
        assert!(matches!(err, LaunchError::TerminalMissing(t) if t == foreign), "{err}");
    }

    #[test]
    fn terminate_refuses_low_pids() {
        assert!(terminate(0).is_err());
        assert!(terminate(1).is_err());
    }

    #[test]
    fn run_with_timeout_kills_a_stuck_child() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "ping -n 30 127.0.0.1 > nul"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("30");
            c
        };
        let started = Instant::now();
        let r = run_with_timeout(&mut cmd, Duration::from_millis(200)).unwrap();
        assert!(r.is_none());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn run_with_timeout_captures_output() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "echo hello"]);
            c
        } else {
            let mut c = Command::new("echo");
            c.arg("hello");
            c
        };
        let r = run_with_timeout(&mut cmd, Duration::from_secs(5)).unwrap().unwrap();
        assert!(r.success());
        assert_eq!(r.stdout_text(), "hello");
    }

    #[test]
    fn spawn_detached_reports_immediate_failure() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "echo boom 1>&2 & exit 3"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "echo boom >&2; exit 3"]);
            c
        };
        let err = spawn_detached(&mut cmd, Duration::from_secs(5), true).unwrap_err();
        assert!(err.to_string().contains("boom"), "{err}");
    }

    #[test]
    fn spawn_detached_hands_off_a_long_running_child() {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "ping -n 2 127.0.0.1 > nul"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("1");
            c
        };
        let started = Instant::now();
        spawn_detached(&mut cmd, Duration::from_millis(100), true).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn terminate_signals_a_real_child() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        terminate(child.id()).unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());
    }

    #[test]
    fn default_terminal_is_installed() {
        let list = installed_terminals();
        assert!(!list.is_empty());
        assert_eq!(default_terminal(), list[0].id);
        if cfg!(target_os = "macos") {
            assert!(list.iter().any(|t| t.id == TerminalApp::Terminal));
        }
        if cfg!(windows) {
            assert!(list.iter().any(|t| t.id == TerminalApp::Cmd));
        }
    }
}
