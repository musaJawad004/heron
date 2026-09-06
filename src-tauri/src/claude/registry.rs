//! Live sessions from `~/.claude/sessions/<pid>.json`.
//!
//! Contract:
//! - `snapshot()` parses every `*.json` in the directory (never `*.key` —
//!   they are not opened at all), maps fields as in docs/ARCHITECTURE.md
//!   (ms timestamps; `last_active_at = max(updatedAt, statusUpdatedAt)`,
//!   falling back to the file mtime) and drops entries whose pid is not
//!   alive or whose process is not a `claude` process (pid reuse).
//! - Liveness (`exists`) is re-checked on every call, so a quit session
//!   disappears within one poll. The "is it claude" verdict is cached per
//!   `(pid, startedAt)` for 30 s and the tty is looked up once per entry.
//! - Fills `Session.tty` on Unix from the process table.
//! - Malformed input is skipped with a debug log; nothing here panics. When
//!   a file is unreadable because the CLI is rewriting it, the last good
//!   parse for that pid is reused for that poll so rows do not flicker.
//! - Sorted newest `started_at` first.

use crate::claude::jsonl::{get_str, get_u64, mtime_ms};
use crate::claude::process::{global_probe, ProcessProbe};
use crate::model::{Session, SessionStatus};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long the "this pid is claude" verdict is trusted.
const VERDICT_TTL: Duration = Duration::from_secs(30);
/// Registry files are a few hundred bytes; refuse to parse anything huge.
const MAX_FILE_BYTES: u64 = 256 * 1024;

pub struct Registry {
    pub dir: PathBuf,
    probe: Arc<dyn ProcessProbe>,
    cache: Mutex<HashMap<u32, PidEntry>>,
}

#[derive(Clone, Debug)]
struct PidEntry {
    started_at: Option<u64>,
    is_claude: bool,
    checked_at: Instant,
    /// `None` = not looked up yet; `Some(None)` = looked up, no terminal.
    tty: Option<Option<String>>,
    /// Last successfully parsed and verified session for this pid.
    last_good: Option<Session>,
}

#[derive(Debug, thiserror::Error)]
enum ParseError {
    #[error("read failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("file larger than {MAX_FILE_BYTES} bytes")]
    TooLarge,
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not a JSON object")]
    NotAnObject,
    #[error("missing sessionId")]
    MissingSessionId,
    #[error("missing pid")]
    MissingPid,
}

impl Registry {
    pub fn new(dir: PathBuf) -> Self {
        Self::with_probe(dir, global_probe())
    }

    /// Use a custom process probe (tests).
    pub fn with_probe(dir: PathBuf, probe: Arc<dyn ProcessProbe>) -> Self {
        Self { dir, probe, cache: Mutex::new(HashMap::new()) }
    }

    /// All live sessions, newest `started_at` first.
    pub fn snapshot(&self) -> Vec<Session> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) => {
                log::debug!("registry dir {} unreadable: {e}", self.dir.display());
                return Vec::new();
            }
        };
        let mut seen: HashSet<u32> = HashSet::new();
        let mut sessions = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
            if !is_file || path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let session = match parse_registry_file(&path) {
                Ok(s) => s,
                Err(e) => {
                    log::debug!("registry file {} skipped: {e}", path.display());
                    match self.last_good_for(&path) {
                        Some(s) => s,
                        None => continue,
                    }
                }
            };
            let Some(pid) = session.pid else { continue };
            seen.insert(pid);
            if let Some(verified) = self.verify(session) {
                sessions.push(verified);
            }
        }
        self.cache.lock().retain(|pid, _| seen.contains(pid));
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at).then_with(|| a.id.cmp(&b.id)));
        sessions
    }

    /// The last verified session for the pid named by this file, if any.
    fn last_good_for(&self, path: &Path) -> Option<Session> {
        let pid = stem_pid(path)?;
        self.cache.lock().get(&pid).and_then(|e| e.last_good.clone())
    }

    /// Drop dead or non-claude pids; fill `tty`; maintain the per-pid cache.
    fn verify(&self, mut session: Session) -> Option<Session> {
        let pid = session.pid?;
        if !self.probe.exists(pid) {
            self.cache.lock().remove(&pid);
            return None;
        }
        let now = Instant::now();
        let previous = self.cache.lock().get(&pid).cloned().filter(|e| e.started_at == session.started_at);
        let fresh = previous.as_ref().filter(|e| now.duration_since(e.checked_at) < VERDICT_TTL);
        let (is_claude, checked_at) = match fresh {
            Some(e) => (e.is_claude, e.checked_at),
            None => (self.probe.is_claude(pid), now),
        };
        let tty = match previous.as_ref().and_then(|e| e.tty.clone()) {
            Some(known) => Some(known),
            None if is_claude => Some(self.probe.tty(pid)),
            None => None,
        };
        session.tty = tty.clone().flatten();
        let entry = PidEntry {
            started_at: session.started_at,
            is_claude,
            checked_at,
            tty,
            last_good: is_claude.then(|| session.clone()),
        };
        self.cache.lock().insert(pid, entry);
        is_claude.then_some(session)
    }
}

/// True if a process with this pid exists (any owner).
pub fn is_pid_alive(pid: u32) -> bool {
    global_probe().exists(pid)
}

/// `<pid>.json` → pid.
fn stem_pid(path: &Path) -> Option<u32> {
    path.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse().ok())
}

/// Map one registry file to a `Session` (unverified: `pid` is whatever the file says).
fn parse_registry_file(path: &Path) -> Result<Session, ParseError> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(ParseError::TooLarge);
    }
    let bytes = std::fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if !value.is_object() {
        return Err(ParseError::NotAnObject);
    }
    let id = get_str(&value, "sessionId").ok_or(ParseError::MissingSessionId)?.to_string();
    let pid = get_u64(&value, "pid")
        .and_then(|p| u32::try_from(p).ok())
        .or_else(|| stem_pid(path))
        .ok_or(ParseError::MissingPid)?;
    let started_at = get_u64(&value, "startedAt");
    let updated = get_u64(&value, "updatedAt");
    let status_updated = get_u64(&value, "statusUpdatedAt");
    let last_active_at = updated
        .into_iter()
        .chain(status_updated)
        .max()
        .or_else(|| mtime_ms(&meta))
        .or(started_at)
        .unwrap_or(0);
    Ok(Session {
        id,
        name: get_str(&value, "name").map(str::to_string),
        title: None,
        cwd: get_str(&value, "cwd").unwrap_or_default().to_string(),
        pid: Some(pid),
        status: SessionStatus::from_raw(get_str(&value, "status")),
        started_at,
        last_active_at,
        version: get_str(&value, "version").map(str::to_string),
        git_branch: None,
        transcript_path: None,
        tty: None,
        attention: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::process::FakeProbe;
    use serde_json::json;
    use std::fs;

    /// Verbatim from docs/ARCHITECTURE.md.
    const DOC_SAMPLE: &str = r#"{"pid":3412,"sessionId":"4e035c9a-1ab6-4026-aed1-0fa8731e7792","cwd":"/Users/adz/civl-mobile-app",
 "startedAt":1788684708093,"procStart":"Sun Sep  6 08:51:47 2026","version":"2.1.263",
 "peerProtocol":1,"peerFeatures":["notify_idle","reply_across_default_dirs","artifact_yield"],
 "kind":"interactive","entrypoint":"cli","pidDomain":"darwin",
 "messagingSocketPath":"/tmp/cc-socks/3412.sock","name":"civl-mobile-app-1b","nameSource":"derived",
 "nameSince":1788684708094,"updatedAt":1788684844819,"status":"idle","statusUpdatedAt":1788684844819,
 "bridgeSessionId":"session_013zRkg3XpHjccY8Bo4FPPmo"}"#;

    struct Fixture {
        dir: tempfile::TempDir,
        probe: Arc<FakeProbe>,
        registry: Registry,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let probe = Arc::new(FakeProbe::new());
        let registry = Registry::with_probe(dir.path().to_path_buf(), probe.clone());
        Fixture { dir, probe, registry }
    }

    impl Fixture {
        fn write(&self, name: &str, body: &str) -> PathBuf {
            let p = self.dir.path().join(name);
            fs::write(&p, body).unwrap();
            p
        }

        fn write_session(&self, pid: u32, id: &str, status: Option<&str>, started: u64, updated: u64) {
            let mut v = json!({
                "pid": pid, "sessionId": id, "cwd": format!("/Users/adz/{id}"), "startedAt": started,
                "updatedAt": updated, "statusUpdatedAt": updated - 1, "version": "2.1.263",
                "name": format!("{id}-1a"), "kind": "interactive", "entrypoint": "cli",
            });
            if let Some(s) = status {
                v["status"] = json!(s);
            }
            self.write(&format!("{pid}.json"), &v.to_string());
        }
    }

    #[test]
    fn parses_documented_sample_with_ms_timestamps() {
        let f = fixture();
        f.write("3412.json", DOC_SAMPLE);
        f.probe.add_claude(3412, Some("ttys003"));
        let snap = f.registry.snapshot();
        assert_eq!(snap.len(), 1);
        let s = &snap[0];
        assert_eq!(s.id, "4e035c9a-1ab6-4026-aed1-0fa8731e7792");
        assert_eq!(s.name.as_deref(), Some("civl-mobile-app-1b"));
        assert_eq!(s.cwd, "/Users/adz/civl-mobile-app");
        assert_eq!(s.pid, Some(3412));
        assert_eq!(s.status, SessionStatus::Idle);
        assert_eq!(s.started_at, Some(1788684708093));
        assert_eq!(s.last_active_at, 1788684844819);
        assert_eq!(s.version.as_deref(), Some("2.1.263"));
        assert_eq!(s.tty.as_deref(), Some("ttys003"));
        assert!(s.is_running());
        assert_eq!(s.title, None);
        assert_eq!(s.attention, None);
    }

    #[test]
    fn maps_every_status_and_unknown_when_missing() {
        let f = fixture();
        let cases = [
            (1, Some("busy")),
            (2, Some("idle")),
            (3, Some("needs_input")),
            (4, Some("waiting")),
            (5, None),
            (6, Some("dancing")),
        ];
        for (pid, status) in cases {
            f.write_session(pid, &format!("id-{pid}"), status, 1_000 + pid as u64, 2_000);
            f.probe.add_claude(pid, None);
        }
        let by_pid: HashMap<u32, SessionStatus> =
            f.registry.snapshot().into_iter().map(|s| (s.pid.unwrap(), s.status)).collect();
        assert_eq!(by_pid[&1], SessionStatus::Busy);
        assert_eq!(by_pid[&2], SessionStatus::Idle);
        assert_eq!(by_pid[&3], SessionStatus::NeedsInput);
        assert_eq!(by_pid[&4], SessionStatus::Waiting);
        assert_eq!(by_pid[&5], SessionStatus::Unknown);
        assert_eq!(by_pid[&6], SessionStatus::Unknown);
    }

    #[test]
    fn last_active_is_max_of_updated_fields_or_mtime() {
        let f = fixture();
        f.write("10.json", r#"{"pid":10,"sessionId":"a","updatedAt":100,"statusUpdatedAt":250}"#);
        f.write("11.json", r#"{"pid":11,"sessionId":"b","startedAt":5}"#);
        f.probe.add_claude(10, None);
        f.probe.add_claude(11, None);
        let snap = f.registry.snapshot();
        let a = snap.iter().find(|s| s.id == "a").unwrap();
        assert_eq!(a.last_active_at, 250);
        let b = snap.iter().find(|s| s.id == "b").unwrap();
        let now = crate::model::now_ms();
        assert!(
            b.last_active_at > now - 60_000 && b.last_active_at <= now + 1_000,
            "mtime fallback: {}",
            b.last_active_at
        );
        assert_eq!(b.cwd, "");
        assert_eq!(b.name, None);
    }

    #[test]
    fn stale_pid_is_dropped_and_noticed_within_one_poll() {
        let f = fixture();
        f.write("42.json", DOC_SAMPLE.replace("\"pid\":3412", "\"pid\":42").as_str());
        assert!(f.registry.snapshot().is_empty(), "no such process");
        f.probe.add_claude(42, None);
        assert_eq!(f.registry.snapshot().len(), 1);
        f.probe.kill(42);
        assert!(f.registry.snapshot().is_empty(), "gone on the very next poll");
        // Reappears with a fresh verdict, not a cached one.
        f.probe.add_claude(42, None);
        assert_eq!(f.registry.snapshot().len(), 1);
        assert_eq!(f.probe.calls().1, 2, "is_claude re-evaluated after the pid died");
    }

    #[test]
    fn non_claude_pid_is_dropped() {
        let f = fixture();
        f.write("77.json", DOC_SAMPLE.replace("\"pid\":3412", "\"pid\":77").as_str());
        f.probe.add_other(77);
        assert!(f.registry.snapshot().is_empty());
        // Still alive but not claude: verdict cached, not re-asked every poll.
        f.registry.snapshot();
        assert_eq!(f.probe.calls().1, 1);
        assert_eq!(f.probe.calls().2, 0, "tty never looked up for non-claude pids");
    }

    #[test]
    fn verdict_and_tty_are_cached_but_liveness_is_not() {
        let f = fixture();
        f.write("3412.json", DOC_SAMPLE);
        f.probe.add_claude(3412, Some("ttys003"));
        for _ in 0..5 {
            let snap = f.registry.snapshot();
            assert_eq!(snap[0].tty.as_deref(), Some("ttys003"));
        }
        assert_eq!(f.probe.calls(), (5, 1, 1));
    }

    #[test]
    fn new_started_at_forces_a_fresh_verdict() {
        let f = fixture();
        f.write_session(9, "s1", Some("busy"), 1_000, 1_500);
        f.probe.add_claude(9, Some("ttys001"));
        assert_eq!(f.registry.snapshot().len(), 1);
        // Same pid, new process (registry rewritten with a later startedAt).
        f.write_session(9, "s2", Some("busy"), 2_000, 2_500);
        f.probe.kill(9);
        f.probe.add_other(9);
        assert!(f.registry.snapshot().is_empty());
        assert_eq!(f.probe.calls().1, 2);
    }

    #[test]
    fn ignores_key_files_other_files_and_malformed_json() {
        let f = fixture();
        f.write("3412.json", DOC_SAMPLE);
        // A .key that would parse as a registry entry if it were ever opened.
        f.write("3412.abc.key", r#"{"pid":3412,"sessionId":"from-the-key-file"}"#);
        f.write("9999.key", r#"{"pid":9999,"sessionId":"also-from-key"}"#);
        f.write("notes.txt", r#"{"pid":3412,"sessionId":"txt"}"#);
        f.write("bad.json", "{ this is not json");
        f.write("empty.json", "");
        f.write("array.json", "[1,2,3]");
        f.write("noid.json", r#"{"pid":5}"#);
        f.write("nopid.json", r#"{"sessionId":"x"}"#);
        fs::create_dir(f.dir.path().join("dir.json")).unwrap();
        f.probe.add_claude(3412, None);
        f.probe.add_claude(9999, None);
        f.probe.add_claude(5, None);
        let snap = f.registry.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].id, "4e035c9a-1ab6-4026-aed1-0fa8731e7792");
    }

    #[test]
    fn pid_falls_back_to_file_stem() {
        let f = fixture();
        f.write("123.json", r#"{"sessionId":"stem-pid","status":"busy"}"#);
        f.probe.add_claude(123, None);
        let snap = f.registry.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].pid, Some(123));
    }

    #[test]
    fn mid_rewrite_file_reuses_last_good_parse() {
        let f = fixture();
        f.write_session(50, "keep-me", Some("busy"), 1_000, 1_500);
        f.probe.add_claude(50, None);
        assert_eq!(f.registry.snapshot().len(), 1);
        f.write("50.json", "{\"pid\":50,\"sessionId\":\"ke");
        let snap = f.registry.snapshot();
        assert_eq!(snap.len(), 1, "torn write does not blank the session");
        assert_eq!(snap[0].id, "keep-me");
        f.probe.kill(50);
        assert!(f.registry.snapshot().is_empty(), "but a dead pid still wins");
    }

    #[test]
    fn sorted_newest_started_first() {
        let f = fixture();
        f.write_session(1, "old", Some("idle"), 1_000, 5_000);
        f.write_session(2, "new", Some("idle"), 3_000, 4_000);
        f.write_session(3, "mid", Some("idle"), 2_000, 9_000);
        f.write("4.json", r#"{"pid":4,"sessionId":"nostart"}"#);
        for pid in 1..=4 {
            f.probe.add_claude(pid, None);
        }
        let ids: Vec<String> = f.registry.snapshot().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["new", "mid", "old", "nostart"]);
    }

    #[test]
    fn windows_style_cwd_is_parsed_unchanged() {
        let f = fixture();
        f.write(
            "3412.json",
            r#"{"pid":3412,"sessionId":"w","cwd":"C:\\Users\\me\\proj","status":"idle","pidDomain":"win32"}"#,
        );
        f.probe.add_claude(3412, None);
        let snap = f.registry.snapshot();
        assert_eq!(snap[0].cwd, r"C:\Users\me\proj");
        assert_eq!(snap[0].project_name(), "proj");
        assert_eq!(snap[0].tty, None);
    }

    #[test]
    fn missing_directory_is_empty_not_a_panic() {
        let probe = Arc::new(FakeProbe::new());
        let registry = Registry::with_probe(PathBuf::from("/definitely/not/here"), probe);
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn snapshot_stays_cheap_with_fake_probe() {
        let f = fixture();
        for pid in 1..=10u32 {
            f.write_session(pid, &format!("id-{pid}"), Some("busy"), pid as u64, 10);
            f.probe.add_claude(pid, Some("ttys001"));
        }
        f.registry.snapshot();
        let t = Instant::now();
        for _ in 0..20 {
            assert_eq!(f.registry.snapshot().len(), 10);
        }
        let per_call = t.elapsed() / 20;
        println!("registry snapshot (10 files, fake probe, warm): {per_call:?} per call");
        assert!(per_call < Duration::from_millis(50), "unexpectedly slow: {per_call:?}");
    }
}
