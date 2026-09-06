//! Watches the spool directory for event files dropped by the hook script.
//!
//! - A `notify` watcher on the directory wakes a worker thread; a 3 s poll
//!   is the backstop when the OS watcher misses or cannot start. Bursts are
//!   debounced (~50 ms). Files already present at start are processed.
//! - `*.json` files are handled in name order (names start with the Unix
//!   seconds the script wrote them): read, parse, delete, callback. Files
//!   over 256 KB, unreadable or unparseable are deleted without a callback;
//!   a file that cannot be deleted is skipped from then on so it never
//!   produces the same event twice. Stale `*.tmp` leftovers are reaped.
//! - `parse` keeps only the `HookEvent` fields (never `tool_input`,
//!   `user_input`, `prompt_id`, `permission_mode`); `session_id` is
//!   required; `message` is truncated to 200 chars on a char boundary.
//! - Dropping the `HookWatcher` stops it; `stop()` also joins the thread.
//!   Payloads are never logged.

use super::fsutil;
use crate::model::{now_ms, HookEvent, HookEventKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

/// Largest spool file Heron reads; the script caps stdin at the same size.
pub const MAX_EVENT_BYTES: u64 = 262_144;
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const DEBOUNCE: Duration = Duration::from_millis(50);
const MESSAGE_CHARS: usize = 200;
/// Subtypes are short enum-like tokens; anything longer is not one.
const SUBTYPE_CHARS: usize = 64;
/// A `.tmp` older than this was abandoned by a killed hook process.
const STALE_TMP: Duration = Duration::from_secs(60);
/// Spool files older than this are deleted unread.
///
/// The hook keeps firing while Heron is not running, and what it spools is the
/// CLI's raw payload, prompt text included. Heron drops that text the moment it
/// parses a file, but an unread file would otherwise sit on disk forever. An
/// event this old is useless anyway: nobody needs to hear that a permission
/// prompt appeared an hour ago.
const STALE_EVENT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing session_id")]
    MissingSessionId,
}

/// Handle that stops the watcher when dropped.
pub struct HookWatcher {
    stop: Arc<AtomicBool>,
    wake: Sender<()>,
    watcher: Option<RecommendedWatcher>,
    thread: Option<JoinHandle<()>>,
}

impl HookWatcher {
    /// Start watching `events_dir` (created 0700 when missing); `on_event`
    /// is called from a background thread, one event at a time.
    pub fn start<F>(events_dir: PathBuf, on_event: F) -> io::Result<Self>
    where
        F: Fn(HookEvent) + Send + 'static,
    {
        Self::start_with(events_dir, POLL_INTERVAL, on_event)
    }

    fn start_with<F>(events_dir: PathBuf, poll: Duration, on_event: F) -> io::Result<Self>
    where
        F: Fn(HookEvent) + Send + 'static,
    {
        fsutil::create_private_dir(&events_dir)?;
        let stop = Arc::new(AtomicBool::new(false));
        let (wake, rx) = mpsc::channel::<()>();
        let watcher = match fs_watcher(&events_dir, wake.clone()) {
            Ok(w) => Some(w),
            Err(e) => {
                log::warn!("filesystem watcher unavailable for {}: {e}; polling only", events_dir.display());
                None
            }
        };
        let worker = Worker {
            dir: events_dir,
            stop: stop.clone(),
            rx,
            poll,
            on_event: Box::new(on_event),
            skip: HashSet::new(),
        };
        let thread = thread::Builder::new().name("heron-hook-watcher".into()).spawn(move || worker.run())?;
        Ok(Self { stop, wake, watcher, thread: Some(thread) })
    }

    /// Stop and wait for the worker thread to finish.
    pub fn stop(mut self) {
        self.signal_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    fn signal_stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.wake.send(());
        self.watcher = None;
    }
}

impl Drop for HookWatcher {
    fn drop(&mut self) {
        self.signal_stop();
    }
}

fn fs_watcher(dir: &Path, wake: Sender<()>) -> notify::Result<RecommendedWatcher> {
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(_) => {
                let _ = wake.send(());
            }
            Err(e) => log::warn!("hook watcher error: {e}"),
        })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

struct Worker {
    dir: PathBuf,
    stop: Arc<AtomicBool>,
    rx: Receiver<()>,
    poll: Duration,
    on_event: Box<dyn Fn(HookEvent) + Send>,
    /// Files that could not be deleted; never processed again.
    skip: HashSet<OsString>,
}

impl Worker {
    fn run(mut self) {
        self.scan();
        while !self.stop.load(Ordering::SeqCst) {
            match self.rx.recv_timeout(self.poll) {
                Ok(()) => {
                    thread::sleep(DEBOUNCE);
                    while self.rx.try_recv().is_ok() {}
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            if self.stop.load(Ordering::SeqCst) {
                break;
            }
            self.scan();
        }
        log::debug!("hook watcher stopped");
    }

    fn scan(&mut self) {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::debug!("events dir unreadable: {e}");
                return;
            }
        };
        let mut files = Vec::new();
        for entry in entries.flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();
            match path.extension().and_then(|e| e.to_str()) {
                Some("json") if !self.skip.contains(&entry.file_name()) => {
                    if older_than(&path, STALE_EVENT) {
                        log::debug!("discarding a hook event that went unread");
                        let _ = fs::remove_file(&path);
                    } else {
                        files.push(path);
                    }
                }
                Some("tmp") => reap_stale_tmp(&path),
                _ => {}
            }
        }
        files.sort();
        for path in files {
            self.consume(&path);
        }
    }

    /// Read + parse one spool file, delete it, then hand the event on.
    fn consume(&mut self, path: &Path) {
        let event = read_event(path);
        if let Err(e) = fs::remove_file(path) {
            if e.kind() != io::ErrorKind::NotFound {
                log::warn!("could not delete hook event file: {e}; ignoring it from now on");
                if let Some(name) = path.file_name() {
                    self.skip.insert(name.to_os_string());
                }
            }
        }
        if let Some(event) = event {
            log::debug!("hook event {} for session {}", event.kind.name(), event.session_id);
            (self.on_event)(event);
        }
    }
}

fn read_event(path: &Path) -> Option<HookEvent> {
    if fs::metadata(path).is_ok_and(|m| m.len() > MAX_EVENT_BYTES) {
        log::warn!("dropping oversized hook event file");
        return None;
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                log::warn!("could not read hook event file: {e}");
            }
            return None;
        }
    };
    match parse(&bytes) {
        Ok(event) => Some(event),
        Err(e) => {
            log::warn!("dropping unparseable hook event: {e}");
            None
        }
    }
}

fn older_than(path: &Path, limit: Duration) -> bool {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age > limit)
}

fn reap_stale_tmp(path: &Path) {
    if older_than(path, STALE_TMP) {
        let _ = fs::remove_file(path);
    }
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Parse the raw stdin JSON that Claude Code handed the hook, lifting only
/// the allowed fields (see docs/HOOKS.md).
pub fn parse(bytes: &[u8]) -> Result<HookEvent, ParseError> {
    let raw: Value = serde_json::from_slice(bytes)?;
    let field = |key: &str| raw.get(key).and_then(Value::as_str).filter(|s| !s.is_empty());
    let session_id = field("session_id").ok_or(ParseError::MissingSessionId)?.to_string();
    let kind = HookEventKind::from_name(field("hook_event_name").unwrap_or_default());
    let subtype = ["notification_type", "reason", "source", "result"]
        .into_iter()
        .find_map(field)
        .map(|s| truncate_chars(s, SUBTYPE_CHARS));
    let message = if kind == HookEventKind::Notification {
        field("message").map(|m| truncate_chars(m, MESSAGE_CHARS))
    } else {
        None
    };
    let now = now_ms();
    Ok(HookEvent {
        id: format!("{now}-{}", COUNTER.fetch_add(1, Ordering::Relaxed)),
        kind,
        session_id,
        cwd: field("cwd").map(String::from),
        transcript_path: field("transcript_path").map(String::from),
        subtype,
        message,
        tool_name: field("tool_name").map(String::from),
        received_at: now,
    })
}

/// The first `max` chars of `s` (never splits a multibyte char).
fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((end, _)) => s[..end].to_string(),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::time::Instant;

    // ---- parse -----------------------------------------------------------

    #[test]
    fn parse_lifts_only_allowed_fields() {
        let raw = br#"{
            "session_id": "sess-1", "hook_event_name": "Notification",
            "cwd": "/Users/me/proj", "transcript_path": "/Users/me/.claude/projects/x/sess-1.jsonl",
            "notification_type": "permission_prompt", "title": "Claude Code",
            "message": "Claude needs your permission to use Bash",
            "tool_name": "Bash", "tool_input": {"command": "rm -rf / SECRET_TOOL_INPUT"},
            "user_input": "SECRET_PROMPT", "prompt_id": "p-1", "permission_mode": "default"
        }"#;
        let ev = parse(raw).unwrap();
        assert_eq!(ev.kind, HookEventKind::Notification);
        assert_eq!(ev.session_id, "sess-1");
        assert_eq!(ev.cwd.as_deref(), Some("/Users/me/proj"));
        assert_eq!(ev.transcript_path.as_deref(), Some("/Users/me/.claude/projects/x/sess-1.jsonl"));
        assert_eq!(ev.subtype.as_deref(), Some("permission_prompt"));
        assert_eq!(ev.message.as_deref(), Some("Claude needs your permission to use Bash"));
        assert_eq!(ev.tool_name.as_deref(), Some("Bash"));
        assert!(ev.received_at > 0);
        assert!(ev.id.starts_with(&format!("{}-", ev.received_at)));
        let serialized = serde_json::to_string(&ev).unwrap();
        for secret in ["SECRET_TOOL_INPUT", "SECRET_PROMPT", "p-1", "permission_mode", "Claude Code"] {
            assert!(!serialized.contains(secret), "{secret} leaked into {serialized}");
        }
    }

    #[test]
    fn parse_subtype_sources_and_kinds() {
        let ev = parse(br#"{"session_id":"s","hook_event_name":"SessionEnd","reason":"user_exit"}"#).unwrap();
        assert_eq!((ev.kind, ev.subtype.as_deref()), (HookEventKind::SessionEnd, Some("user_exit")));
        let ev = parse(br#"{"session_id":"s","hook_event_name":"SessionStart","source":"resume"}"#).unwrap();
        assert_eq!((ev.kind, ev.subtype.as_deref()), (HookEventKind::SessionStart, Some("resume")));
        let ev = parse(br#"{"session_id":"s","hook_event_name":"SubagentStop","result":"ok"}"#).unwrap();
        assert_eq!((ev.kind, ev.subtype.as_deref()), (HookEventKind::SubagentStop, Some("ok")));
        let ev = parse(br#"{"session_id":"s","hook_event_name":"PermissionRequest","tool_name":"Edit","tool_input":{"file_path":"x"}}"#).unwrap();
        assert_eq!(
            (ev.kind, ev.tool_name.as_deref(), ev.subtype),
            (HookEventKind::PermissionRequest, Some("Edit"), None)
        );
        let ev = parse(br#"{"session_id":"s","hook_event_name":"Stop"}"#).unwrap();
        assert_eq!((ev.kind, ev.subtype, ev.message, ev.tool_name), (HookEventKind::Stop, None, None, None));
        let ev = parse(br#"{"session_id":"s","hook_event_name":"Whatever","message":"not a notification"}"#)
            .unwrap();
        assert_eq!((ev.kind, ev.message), (HookEventKind::Other, None));
        let ev = parse(br#"{"session_id":"s"}"#).unwrap();
        assert_eq!(ev.kind, HookEventKind::Other);
        let ev =
            parse(br#"{"session_id":"s","hook_event_name":"SessionEnd","reason":12,"cwd":null}"#).unwrap();
        assert_eq!((ev.subtype, ev.cwd), (None, None), "non-string fields are ignored");
    }

    #[test]
    fn parse_requires_session_id() {
        assert!(matches!(parse(br#"{"hook_event_name":"Stop"}"#), Err(ParseError::MissingSessionId)));
        assert!(matches!(
            parse(br#"{"session_id":"","hook_event_name":"Stop"}"#),
            Err(ParseError::MissingSessionId)
        ));
        assert!(matches!(parse(br#"{"session_id":7}"#), Err(ParseError::MissingSessionId)));
        assert!(matches!(parse(b"{not json"), Err(ParseError::Json(_))));
        assert!(matches!(parse(b""), Err(ParseError::Json(_))));
        assert!(matches!(parse(b"[1,2]"), Err(ParseError::MissingSessionId)));
    }

    #[test]
    fn parse_truncates_message_on_char_boundary() {
        let message: String = "é".repeat(150) + &"日".repeat(150);
        let raw = serde_json::json!({
            "session_id": "s", "hook_event_name": "Notification",
            "notification_type": "idle_prompt", "message": message,
        });
        let ev = parse(serde_json::to_vec(&raw).unwrap().as_slice()).unwrap();
        let got = ev.message.unwrap();
        assert_eq!(got.chars().count(), 200);
        assert_eq!(got, "é".repeat(150) + &"日".repeat(50));
        let short = parse(br#"{"session_id":"s","hook_event_name":"Notification","message":"hi"}"#).unwrap();
        assert_eq!(short.message.as_deref(), Some("hi"));
    }

    #[test]
    fn parse_ids_are_unique() {
        let a = parse(br#"{"session_id":"s"}"#).unwrap();
        let b = parse(br#"{"session_id":"s"}"#).unwrap();
        assert_ne!(a.id, b.id);
    }

    // ---- watcher ---------------------------------------------------------

    type Seen = Arc<Mutex<Vec<HookEvent>>>;

    fn start(dir: &Path) -> (HookWatcher, Seen) {
        let seen: Seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let watcher = HookWatcher::start(dir.to_path_buf(), move |ev| sink.lock().push(ev)).unwrap();
        (watcher, seen)
    }

    /// Drop a spool file the way the script does: temp file, then rename.
    fn spool(dir: &Path, name: &str, bytes: &[u8]) {
        let tmp = dir.join(format!(".{name}.tmp"));
        fs::write(&tmp, bytes).unwrap();
        fs::rename(tmp, dir.join(name)).unwrap();
    }

    fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if done() {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        done()
    }

    #[test]
    fn watcher_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let events = dir.path().join("events");
        // Pre-existing files are processed at start, in name order.
        fs::create_dir_all(&events).unwrap();
        spool(&events, "1700000002-2.json", br#"{"session_id":"early","hook_event_name":"Stop"}"#);
        spool(&events, "1700000001-1.json", br#"{"session_id":"earlier","hook_event_name":"SessionStart"}"#);

        let (watcher, seen) = start(&events);
        assert!(wait_until(Duration::from_secs(2), || seen.lock().len() == 2));
        {
            let seen = seen.lock();
            assert_eq!(seen[0].session_id, "earlier");
            assert_eq!(seen[1].session_id, "early");
        }
        assert_eq!(fs::read_dir(&events).unwrap().count(), 0, "spool files deleted");

        // A new file triggers the callback promptly and is deleted.
        spool(&events, "1700000003-3.json", br#"{"session_id":"live","hook_event_name":"Notification","notification_type":"permission_prompt"}"#);
        assert!(wait_until(Duration::from_secs(2), || seen.lock().len() == 3), "callback within 2 s");
        assert_eq!(seen.lock()[2].subtype.as_deref(), Some("permission_prompt"));
        assert!(!events.join("1700000003-3.json").exists());

        // Oversized and unparseable files are deleted without a callback.
        spool(&events, "1700000004-4.json", &vec![b'{'; MAX_EVENT_BYTES as usize + 1]);
        spool(&events, "1700000005-5.json", b"garbage");
        spool(&events, "1700000006-6.json", br#"{"hook_event_name":"Stop"}"#);
        assert!(wait_until(Duration::from_secs(2), || {
            fs::read_dir(&events)
                .unwrap()
                .filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "json"))
                .count()
                == 0
        }));
        assert_eq!(seen.lock().len(), 3, "no callback for bad files");

        // Stopping joins the thread; nothing is processed afterwards.
        watcher.stop();
        spool(&events, "1700000007-7.json", br#"{"session_id":"late","hook_event_name":"Stop"}"#);
        assert!(!wait_until(Duration::from_millis(400), || seen.lock().len() > 3));
        assert!(events.join("1700000007-7.json").exists());
    }

    #[test]
    fn watcher_creates_dir_and_stops_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let events = dir.path().join("missing").join("events");
        let (watcher, seen) = start(&events);
        assert!(events.is_dir());
        #[cfg(unix)]
        assert_eq!(fsutil::mode_of(&events), Some(0o700));
        let stop = watcher.stop.clone();
        drop(watcher);
        assert!(stop.load(Ordering::SeqCst));
        thread::sleep(Duration::from_millis(100));
        spool(&events, "1700000001-1.json", br#"{"session_id":"x","hook_event_name":"Stop"}"#);
        assert!(!wait_until(Duration::from_millis(300), || !seen.lock().is_empty()));
    }

    #[test]
    fn poll_backstop_picks_up_files_without_fs_events() {
        let dir = tempfile::tempdir().unwrap();
        let events = dir.path().join("events");
        let seen: Seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let mut watcher = HookWatcher::start_with(events.clone(), Duration::from_millis(200), move |ev| {
            sink.lock().push(ev)
        })
        .unwrap();
        // Disable the OS watcher; only the poll remains.
        watcher.watcher = None;
        thread::sleep(Duration::from_millis(50));
        spool(&events, "1700000001-1.json", br#"{"session_id":"polled","hook_event_name":"Stop"}"#);
        assert!(wait_until(Duration::from_secs(2), || seen.lock().len() == 1));
        watcher.stop();
    }

    #[test]
    fn an_unread_event_older_than_the_window_is_discarded_not_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        spool(dir.path(), "1-1.json", br#"{"hook_event_name":"Stop","session_id":"s"}"#);
        let old = dir.path().join("1-1.json");
        // Backdate it past the window: the hook fires while Heron is not
        // running, and what it leaves behind is the CLI's raw payload.
        let stale = SystemTime::now() - STALE_EVENT - Duration::from_secs(30);
        filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(stale)).unwrap();
        spool(dir.path(), "2-2.json", br#"{"hook_event_name":"Stop","session_id":"fresh"}"#);

        let _w = HookWatcher::start(dir.path().to_path_buf(), move |e| {
            let _ = tx.send(e);
        })
        .unwrap();

        let got = rx.recv_timeout(Duration::from_secs(3)).expect("the fresh event");
        assert_eq!(got.session_id, "fresh");
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "the stale one is not parsed");
        assert!(!old.exists(), "and it is deleted");
    }
}
