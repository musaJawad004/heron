//! Process-table lookups behind an injectable trait so `registry` can be
//! unit-tested without live processes.
//!
//! Contract:
//! - `exists(pid)`: does *any* process with this pid exist right now. Cheap
//!   enough to call once per session per second (`/proc/<pid>` on Linux, a
//!   single-pid `sysinfo` refresh elsewhere; never spawns a process).
//! - `is_claude(pid)`: the process name, executable path or command line
//!   mentions "claude" (case-insensitive). Covers the native `claude` binary,
//!   `claude.exe`, and `node`/`bun` running an npm-installed `cli.js`.
//! - `tty(pid)`: controlling terminal from `/bin/ps` on Unix (`None` when the
//!   process has none); always `None` on Windows. This spawns a child, so
//!   callers cache the answer per pid.
//! - `SysinfoProbe` is the real implementation; `FakeProbe` (tests only) is a
//!   hand-filled table that also counts calls so caching can be asserted.

use parking_lot::Mutex;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub trait ProcessProbe: Send + Sync {
    /// True if a process with this pid exists (any owner).
    fn exists(&self, pid: u32) -> bool;
    /// True if the process looks like a Claude Code CLI.
    fn is_claude(&self, pid: u32) -> bool;
    /// Controlling terminal (e.g. `ttys003`, `pts/2`); Unix only.
    fn tty(&self, pid: u32) -> Option<String>;
}

/// The one process table shared by everything in the app.
pub fn global_probe() -> Arc<SysinfoProbe> {
    static PROBE: OnceLock<Arc<SysinfoProbe>> = OnceLock::new();
    PROBE.get_or_init(|| Arc::new(SysinfoProbe::new())).clone()
}

/// Real probe backed by `sysinfo` (and `/proc` for existence on Linux).
pub struct SysinfoProbe {
    system: Mutex<System>,
}

impl Default for SysinfoProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl SysinfoProbe {
    pub fn new() -> Self {
        Self { system: Mutex::new(System::new()) }
    }

    /// Refresh exactly this pid (removing it from the table if it is gone)
    /// and report whether it is still there.
    fn refresh_one(system: &mut System, pid: Pid, kind: ProcessRefreshKind) -> bool {
        system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, kind);
        system.process(pid).is_some()
    }
}

impl ProcessProbe for SysinfoProbe {
    fn exists(&self, pid: u32) -> bool {
        #[cfg(target_os = "linux")]
        {
            std::fs::metadata(format!("/proc/{pid}")).is_ok()
        }
        #[cfg(not(target_os = "linux"))]
        {
            let mut system = self.system.lock();
            Self::refresh_one(&mut system, Pid::from_u32(pid), ProcessRefreshKind::nothing().without_tasks())
        }
    }

    fn is_claude(&self, pid: u32) -> bool {
        let kind = ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet);
        let pid = Pid::from_u32(pid);
        let mut system = self.system.lock();
        if !Self::refresh_one(&mut system, pid, kind) {
            return false;
        }
        system.process(pid).map(|p| looks_like_claude(p.name(), p.exe(), p.cmd())).unwrap_or(false)
    }

    fn tty(&self, pid: u32) -> Option<String> {
        ps_tty(pid)
    }
}

/// The recognition rule, split out so it can be tested without a process.
pub fn looks_like_claude(name: &OsStr, exe: Option<&Path>, cmd: &[OsString]) -> bool {
    fn mentions_claude(s: &OsStr) -> bool {
        s.to_string_lossy().to_ascii_lowercase().contains("claude")
    }
    mentions_claude(name)
        || exe.map(|p| mentions_claude(p.as_os_str())).unwrap_or(false)
        || cmd.iter().any(|arg| mentions_claude(arg))
}

/// `ps -o tty= -p <pid>`, trimmed; `??`/`?`/`-`/empty mean "no terminal".
#[cfg(unix)]
fn ps_tty(pid: u32) -> Option<String> {
    use std::process::Command;
    let pid_arg = pid.to_string();
    let run = |program: &str| Command::new(program).args(["-o", "tty=", "-p", &pid_arg]).output();
    let output = run("/bin/ps").or_else(|_| run("ps")).ok()?;
    if !output.status.success() {
        return None;
    }
    let tty = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match tty.as_str() {
        "" | "??" | "?" | "-" => None,
        _ => Some(tty),
    }
}

#[cfg(not(unix))]
fn ps_tty(_pid: u32) -> Option<String> {
    None
}

/// Test double: a hand-filled process table that counts calls.
#[cfg(test)]
pub struct FakeProbe {
    inner: Mutex<FakeState>,
}

#[cfg(test)]
#[derive(Default)]
struct FakeState {
    alive: std::collections::HashSet<u32>,
    claude: std::collections::HashSet<u32>,
    ttys: std::collections::HashMap<u32, String>,
    exists_calls: usize,
    is_claude_calls: usize,
    tty_calls: usize,
}

#[cfg(test)]
impl Default for FakeProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl FakeProbe {
    pub fn new() -> Self {
        Self { inner: Mutex::new(FakeState::default()) }
    }

    /// A live `claude` process, optionally attached to a terminal.
    pub fn add_claude(&self, pid: u32, tty: Option<&str>) {
        let mut s = self.inner.lock();
        s.alive.insert(pid);
        s.claude.insert(pid);
        if let Some(t) = tty {
            s.ttys.insert(pid, t.to_string());
        }
    }

    /// A live process that is not Claude (pid reuse).
    pub fn add_other(&self, pid: u32) {
        let mut s = self.inner.lock();
        s.alive.insert(pid);
        s.claude.remove(&pid);
    }

    pub fn kill(&self, pid: u32) {
        let mut s = self.inner.lock();
        s.alive.remove(&pid);
        s.claude.remove(&pid);
        s.ttys.remove(&pid);
    }

    /// `(exists, is_claude, tty)` call counts so far.
    pub fn calls(&self) -> (usize, usize, usize) {
        let s = self.inner.lock();
        (s.exists_calls, s.is_claude_calls, s.tty_calls)
    }
}

#[cfg(test)]
impl ProcessProbe for FakeProbe {
    fn exists(&self, pid: u32) -> bool {
        let mut s = self.inner.lock();
        s.exists_calls += 1;
        s.alive.contains(&pid)
    }

    fn is_claude(&self, pid: u32) -> bool {
        let mut s = self.inner.lock();
        s.is_claude_calls += 1;
        s.claude.contains(&pid)
    }

    fn tty(&self, pid: u32) -> Option<String> {
        let mut s = self.inner.lock();
        s.tty_calls += 1;
        s.ttys.get(&pid).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn os(s: &str) -> OsString {
        OsString::from(s)
    }

    #[test]
    fn recognises_native_binary_by_name() {
        assert!(looks_like_claude(OsStr::new("claude"), None, &[]));
        assert!(looks_like_claude(OsStr::new("claude.exe"), None, &[]));
        assert!(looks_like_claude(OsStr::new("CLAUDE"), None, &[]));
    }

    #[test]
    fn recognises_npm_install_by_command_line() {
        let cmd = [os("node"), os("/usr/lib/node_modules/@anthropic-ai/claude-code/cli.js")];
        assert!(looks_like_claude(OsStr::new("node"), Some(Path::new("/usr/bin/node")), &cmd));
        let win = [
            os("node.exe"),
            os(r"C:\Users\me\AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code\cli.js"),
        ];
        assert!(looks_like_claude(OsStr::new("node.exe"), None, &win));
        let bun = [os("bun"), os("run"), os("claude")];
        assert!(looks_like_claude(OsStr::new("bun"), None, &bun));
    }

    #[test]
    fn recognises_by_executable_path_only() {
        let exe = PathBuf::from("/Users/me/.local/bin/claude");
        assert!(looks_like_claude(OsStr::new("versions"), Some(&exe), &[]));
    }

    #[test]
    fn rejects_unrelated_process() {
        let cmd = [os("node"), os("/srv/app/server.js")];
        assert!(!looks_like_claude(OsStr::new("node"), Some(Path::new("/usr/bin/node")), &cmd));
        assert!(!looks_like_claude(OsStr::new("zsh"), None, &[]));
    }

    #[test]
    fn real_probe_sees_our_own_process_and_not_an_absurd_pid() {
        let probe = SysinfoProbe::new();
        let me = std::process::id();
        assert!(probe.exists(me), "our own pid must exist");
        // pid_max on Linux is 4194304; macOS and Windows stay far below this.
        assert!(!probe.exists(u32::MAX - 7));
        // The test binary is not claude.
        assert!(!probe.is_claude(u32::MAX - 7));
    }

    #[test]
    fn fake_probe_counts_calls() {
        let fake = FakeProbe::new();
        fake.add_claude(7, Some("ttys001"));
        fake.add_other(8);
        assert!(fake.exists(7) && fake.is_claude(7));
        assert!(fake.exists(8) && !fake.is_claude(8));
        assert_eq!(fake.tty(7).as_deref(), Some("ttys001"));
        assert_eq!(fake.tty(8), None);
        fake.kill(7);
        assert!(!fake.exists(7));
        assert_eq!(fake.calls(), (3, 2, 2));
    }
}
