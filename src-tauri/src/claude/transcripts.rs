//! Historical (resumable) sessions from `~/.claude/projects/**/*.jsonl` and
//! `~/.claude/history.jsonl`, enriched with a title (first user prompt).
//!
//! Contract:
//! - Candidates come from two cheap sources, unioned by session id:
//!   `history.jsonl` (newest `timestamp` and its `project` per id; only the
//!   last 8 MB are read and `display` is never looked at) and the listing of
//!   `projects/*/<id>.jsonl` (mtime = recency). Anything under a `subagents/`
//!   directory or named `agent-*.jsonl` is ignored. `sessions/` is never read.
//! - For the newest `limit` (and in `enrich`) only the first 64 KB of the
//!   transcript is read to find `cwd`, `gitBranch`, `version` and the first
//!   real user prompt (see `title`).
//! - `cache_dir/transcripts.json` maps `path -> {mtime_ms, size, title, cwd,
//!   git_branch, version}`; reused on `(mtime, size)` match, written
//!   atomically (tmp + rename, mode 0600), capped at the 500 newest entries.
//!   It never contains message bodies.
//! - The id → path map is cached in memory and refreshed when the projects
//!   directory or any project subdirectory changes mtime, or after
//!   `invalidate()`.
//! - `recent_sessions` sorts by `last_active_at` desc and returns at most
//!   `limit` rows with `status: Unknown`, `pid: None`.

use crate::claude::jsonl::{get_str, get_u64, mtime_ms, read_head, read_tail, HeadInfo};
use crate::model::{Session, SessionStatus};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_TAIL_BYTES: u64 = 8 * 1024 * 1024;
const CACHE_FILE: &str = "transcripts.json";
const CACHE_CAP: usize = 500;
const CACHE_VERSION: u32 = 1;

pub struct TranscriptIndex {
    pub projects_dir: PathBuf,
    pub history_file: PathBuf,
    pub cache_dir: PathBuf,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    listing: Option<Listing>,
    /// `None` until loaded from disk.
    cache: Option<HashMap<String, CacheEntry>>,
    dirty: bool,
}

struct Listing {
    signature: Vec<(OsString, Option<SystemTime>)>,
    /// session id → transcript path
    paths: HashMap<String, PathBuf>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct CacheEntry {
    mtime_ms: u64,
    size: u64,
    title: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    version: Option<String>,
}

impl CacheEntry {
    fn matches(&self, mtime_ms: u64, size: u64) -> bool {
        self.mtime_ms == mtime_ms && self.size == size
    }

    fn head(&self) -> HeadInfo {
        HeadInfo {
            title: self.title.clone(),
            cwd: self.cwd.clone(),
            git_branch: self.git_branch.clone(),
            version: self.version.clone(),
        }
    }
}

#[derive(Deserialize)]
struct CacheFile {
    version: u32,
    entries: HashMap<String, CacheEntry>,
}

#[derive(Serialize)]
struct CacheFileRef<'a> {
    version: u32,
    entries: &'a HashMap<String, CacheEntry>,
}

struct Candidate {
    id: String,
    path: Option<PathBuf>,
    last_active_at: u64,
    cwd: Option<String>,
    mtime_ms: u64,
    size: u64,
}

struct HistoryEntry {
    timestamp: u64,
    project: Option<String>,
}

impl TranscriptIndex {
    pub fn new(projects_dir: PathBuf, history_file: PathBuf, cache_dir: PathBuf) -> Self {
        Self { projects_dir, history_file, cache_dir, inner: Mutex::new(Inner::default()) }
    }

    /// Newest `limit` sessions known from disk.
    pub fn recent_sessions(&self, limit: usize) -> Vec<Session> {
        if limit == 0 {
            return Vec::new();
        }
        let paths = self.with_paths(|p| p.clone());
        let mut candidates: HashMap<String, Candidate> = HashMap::with_capacity(paths.len());
        for (id, path) in paths {
            let Ok(meta) = fs::metadata(&path) else { continue };
            let mtime = mtime_ms(&meta).unwrap_or(0);
            let c = Candidate {
                id: id.clone(),
                path: Some(path),
                last_active_at: mtime,
                cwd: None,
                mtime_ms: mtime,
                size: meta.len(),
            };
            candidates.insert(id, c);
        }
        for (id, h) in read_history(&self.history_file) {
            match candidates.get_mut(&id) {
                Some(c) => {
                    c.last_active_at = c.last_active_at.max(h.timestamp);
                    if c.cwd.is_none() {
                        c.cwd = h.project;
                    }
                }
                None => {
                    let c = Candidate {
                        id: id.clone(),
                        path: None,
                        last_active_at: h.timestamp,
                        cwd: h.project,
                        mtime_ms: 0,
                        size: 0,
                    };
                    candidates.insert(id, c);
                }
            }
        }
        let mut candidates: Vec<Candidate> = candidates.into_values().collect();
        candidates.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at).then_with(|| a.id.cmp(&b.id)));
        candidates.truncate(limit);
        let sessions = candidates.into_iter().map(|c| self.build_session(c)).collect();
        self.flush_cache();
        sessions
    }

    /// Fill `title`, `git_branch`, `version`, `transcript_path` (and `cwd` if empty).
    pub fn enrich(&self, mut session: Session) -> Session {
        let Some(path) = self.locate(&session.id) else { return session };
        let Ok(meta) = fs::metadata(&path) else { return session };
        let info = self.head_info(&path, mtime_ms(&meta).unwrap_or(0), meta.len());
        if session.title.is_none() {
            session.title = info.title;
        }
        if session.git_branch.is_none() {
            session.git_branch = info.git_branch;
        }
        if session.version.is_none() {
            session.version = info.version;
        }
        if session.cwd.is_empty() {
            if let Some(cwd) = info.cwd {
                session.cwd = cwd;
            }
        }
        session.transcript_path = Some(path.to_string_lossy().into_owned());
        self.flush_cache();
        session
    }

    /// Forget in-memory state so the next call rescans (unsaved cache
    /// entries are flushed first).
    pub fn invalidate(&self) {
        self.flush_cache();
        let mut inner = self.inner.lock();
        inner.listing = None;
        inner.cache = None;
    }

    // ---- listing ---------------------------------------------------------

    /// Path of the transcript for `id`, if one exists.
    fn locate(&self, id: &str) -> Option<PathBuf> {
        if id.is_empty() {
            return None;
        }
        self.with_paths(|p| p.get(id).cloned())
    }

    /// Run `f` on the up-to-date id → path map (rescans when the directory tree changed).
    fn with_paths<R>(&self, f: impl FnOnce(&HashMap<String, PathBuf>) -> R) -> R {
        let signature = self.signature();
        {
            let inner = self.inner.lock();
            if let Some(listing) = inner.listing.as_ref().filter(|l| l.signature == signature) {
                return f(&listing.paths);
            }
        }
        let paths = scan_projects(&self.projects_dir);
        let mut inner = self.inner.lock();
        let listing = inner.listing.insert(Listing { signature, paths });
        f(&listing.paths)
    }

    /// mtimes of the projects dir and every project subdir; a change means "rescan".
    fn signature(&self) -> Vec<(OsString, Option<SystemTime>)> {
        let root_mtime = fs::metadata(&self.projects_dir).and_then(|m| m.modified()).ok();
        let mut sig = vec![(OsString::from("."), root_mtime)];
        if let Ok(entries) = fs::read_dir(&self.projects_dir) {
            for e in entries.flatten() {
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    sig.push((e.file_name(), e.metadata().and_then(|m| m.modified()).ok()));
                }
            }
        }
        sig.sort();
        sig
    }

    // ---- head info + cache -----------------------------------------------

    fn build_session(&self, c: Candidate) -> Session {
        let info = match &c.path {
            Some(p) => self.head_info(p, c.mtime_ms, c.size),
            None => HeadInfo::default(),
        };
        Session {
            id: c.id,
            name: None,
            title: info.title,
            cwd: info.cwd.or(c.cwd).unwrap_or_default(),
            pid: None,
            status: SessionStatus::Unknown,
            started_at: None,
            last_active_at: c.last_active_at,
            version: info.version,
            git_branch: info.git_branch,
            transcript_path: c.path.map(|p| p.to_string_lossy().into_owned()),
            tty: None,
            attention: None,
        }
    }

    /// Cached head facts for a transcript, re-read when `(mtime, size)` changed.
    fn head_info(&self, path: &Path, mtime_ms: u64, size: u64) -> HeadInfo {
        let key = path.to_string_lossy().into_owned();
        {
            let mut inner = self.inner.lock();
            if let Some(entry) = self.ensure_cache(&mut inner).get(&key) {
                if entry.matches(mtime_ms, size) {
                    return entry.head();
                }
            }
        }
        let info = match read_head(path) {
            Ok(info) => info,
            Err(e) => {
                log::debug!("transcript {} unreadable: {e}", path.display());
                return HeadInfo::default();
            }
        };
        let entry = CacheEntry {
            mtime_ms,
            size,
            title: info.title.clone(),
            cwd: info.cwd.clone(),
            git_branch: info.git_branch.clone(),
            version: info.version.clone(),
        };
        let mut inner = self.inner.lock();
        self.ensure_cache(&mut inner).insert(key, entry);
        inner.dirty = true;
        info
    }

    fn cache_path(&self) -> PathBuf {
        self.cache_dir.join(CACHE_FILE)
    }

    fn ensure_cache<'a>(&self, inner: &'a mut Inner) -> &'a mut HashMap<String, CacheEntry> {
        inner.cache.get_or_insert_with(|| load_cache(&self.cache_path()))
    }

    /// Persist the cache if anything changed (outside the lock).
    fn flush_cache(&self) {
        let entries = {
            let mut inner = self.inner.lock();
            if !inner.dirty {
                return;
            }
            inner.dirty = false;
            let Some(cache) = inner.cache.as_mut() else { return };
            prune(cache);
            cache.clone()
        };
        if let Err(e) = write_cache(&self.cache_path(), &entries) {
            log::warn!("could not write transcript cache: {e}");
        }
    }
}

// ---- free helpers -----------------------------------------------------------

/// `projects/*/<id>.jsonl` → id → path. Sub-agent transcripts are skipped.
fn scan_projects(projects_dir: &Path) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let Ok(projects) = fs::read_dir(projects_dir) else { return out };
    for project in projects.flatten() {
        if !project.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(files) = fs::read_dir(project.path()) else { continue };
        for file in files.flatten() {
            if !file.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = file.path();
            if let Some(id) = transcript_id(&path) {
                out.insert(id, path);
            }
        }
    }
    out
}

/// Session id for a top-level transcript path; `None` for anything else.
fn transcript_id(path: &Path) -> Option<String> {
    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return None;
    }
    if path.components().any(|c| c.as_os_str() == "subagents") {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    if stem.is_empty() || stem.starts_with("agent-") {
        return None;
    }
    Some(stem.to_string())
}

/// Newest timestamp and its `project` per session id. `display` is never read.
fn read_history(path: &Path) -> HashMap<String, HistoryEntry> {
    let mut out: HashMap<String, HistoryEntry> = HashMap::new();
    let text = match read_tail(path, HISTORY_TAIL_BYTES) {
        Ok(t) => t,
        Err(e) => {
            log::debug!("history {} unreadable: {e}", path.display());
            return out;
        }
    };
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else { continue };
        let Some(id) = get_str(&value, "sessionId") else { continue };
        let timestamp = get_u64(&value, "timestamp").unwrap_or(0);
        let project = get_str(&value, "project").map(str::to_string);
        let entry = out.entry(id.to_string()).or_insert(HistoryEntry { timestamp: 0, project: None });
        if timestamp >= entry.timestamp {
            entry.timestamp = timestamp;
            if project.is_some() {
                entry.project = project;
            }
        } else if entry.project.is_none() {
            entry.project = project;
        }
    }
    out
}

fn load_cache(path: &Path) -> HashMap<String, CacheEntry> {
    let Ok(bytes) = fs::read(path) else { return HashMap::new() };
    match serde_json::from_slice::<CacheFile>(&bytes) {
        Ok(file) if file.version == CACHE_VERSION => file.entries,
        Ok(_) => HashMap::new(),
        Err(e) => {
            log::debug!("transcript cache {} ignored: {e}", path.display());
            HashMap::new()
        }
    }
}

/// Keep the `CACHE_CAP` newest entries by transcript mtime.
fn prune(cache: &mut HashMap<String, CacheEntry>) {
    if cache.len() <= CACHE_CAP {
        return;
    }
    let mut keys: Vec<(u64, String)> = cache.iter().map(|(k, e)| (e.mtime_ms, k.clone())).collect();
    keys.sort_unstable_by(|a, b| b.cmp(a));
    for (_, key) in keys.into_iter().skip(CACHE_CAP) {
        cache.remove(&key);
    }
}

/// tmp + rename, 0600 on Unix, directory created 0700 if missing.
fn write_cache(path: &Path, entries: &HashMap<String, CacheEntry>) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| io::Error::other("cache path has no parent"))?;
    create_private_dir(dir)?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let tmp = dir.join(format!("{CACHE_FILE}.{}.{nanos}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&tmp)?;
        let body = serde_json::to_vec(&CacheFileRef { version: CACHE_VERSION, entries })?;
        file.write_all(&body)?;
        file.sync_data()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn create_private_dir(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{Duration, Instant};

    const SECRET_BODY: &str = "SECRET-ASSISTANT-BODY-MUST-NOT-BE-CACHED";

    /// Exact line shapes from docs/ARCHITECTURE.md.
    fn user_line(text: &str, cwd: &str, id: &str) -> String {
        serde_json::json!({
            "parentUuid": null, "isSidechain": false, "promptId": "p", "type": "user",
            "message": {"role": "user", "content": text},
            "timestamp": "2026-09-04T13:37:41.586Z", "uuid": "u", "cwd": cwd, "sessionId": id,
            "version": "2.1.260", "gitBranch": "main"
        })
        .to_string()
    }

    fn command_line(id: &str, cwd: &str) -> String {
        user_line("<command-name>/resume</command-name>", cwd, id)
    }

    fn assistant_line(cwd: &str) -> String {
        serde_json::json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":SECRET_BODY}]},"cwd":cwd}).to_string()
    }

    fn preamble(id: &str) -> Vec<String> {
        vec![
            format!(r#"{{"type":"mode","mode":"normal","sessionId":"{id}"}}"#),
            format!(r#"{{"type":"permission-mode","permissionMode":"auto","sessionId":"{id}"}}"#),
        ]
    }

    struct Fixture {
        tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            fs::create_dir_all(tmp.path().join("projects")).unwrap();
            Self { tmp }
        }

        fn projects(&self) -> PathBuf {
            self.tmp.path().join("projects")
        }

        fn index(&self) -> TranscriptIndex {
            TranscriptIndex::new(
                self.projects(),
                self.tmp.path().join("history.jsonl"),
                self.tmp.path().join("cache"),
            )
        }

        fn write_transcript(&self, encoded: &str, id: &str, lines: &[String]) -> PathBuf {
            let dir = self.projects().join(encoded);
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("{id}.jsonl"));
            fs::write(&path, lines.join("\n") + "\n").unwrap();
            path
        }

        fn simple_transcript(&self, encoded: &str, id: &str, prompt: &str, cwd: &str) -> PathBuf {
            let mut lines = preamble(id);
            lines.push(command_line(id, cwd));
            lines.push(user_line(prompt, cwd, id));
            lines.push(assistant_line(cwd));
            self.write_transcript(encoded, id, &lines)
        }

        fn write_history(&self, lines: &[String]) {
            fs::write(self.tmp.path().join("history.jsonl"), lines.join("\n") + "\n").unwrap();
        }

        fn set_mtime(&self, path: &Path, t: SystemTime) {
            OpenOptions::new().write(true).open(path).unwrap().set_modified(t).unwrap();
        }
    }

    fn history_line(id: &str, ts: u64, project: &str) -> String {
        format!(
            r#"{{"display":"{SECRET_BODY}","pastedContents":{{}},"timestamp":{ts},"project":"{project}","sessionId":"{id}"}}"#
        )
    }

    #[test]
    fn recent_sessions_merge_listing_and_history() {
        let f = Fixture::new();
        let p1 = f.simple_transcript("-Users-adz-alpha", "id-1", "Refactor the parser", "/Users/adz/alpha");
        let p2 = f.simple_transcript("-Users-adz-beta", "id-2", "Write docs", "/Users/adz/beta");
        let base = SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        f.set_mtime(&p1, base + Duration::from_secs(10));
        f.set_mtime(&p2, base + Duration::from_secs(20));
        f.write_history(&[
            history_line("id-1", 1_700_000_050_000, "/Users/adz/alpha"),
            "garbage line".to_string(),
            history_line("id-3", 1_700_000_005_000, "/Users/adz/gone"),
            r#"{"display":"no session id","timestamp":1}"#.to_string(),
        ]);
        let recent = f.index().recent_sessions(10);
        let ids: Vec<&str> = recent.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["id-1", "id-2", "id-3"]);

        let s1 = &recent[0];
        assert_eq!(s1.last_active_at, 1_700_000_050_000, "history newer than mtime wins");
        assert_eq!(s1.title.as_deref(), Some("Refactor the parser"));
        assert_eq!(s1.cwd, "/Users/adz/alpha");
        assert_eq!(s1.git_branch.as_deref(), Some("main"));
        assert_eq!(s1.version.as_deref(), Some("2.1.260"));
        assert_eq!(s1.transcript_path.as_deref(), Some(p1.to_str().unwrap()));
        assert_eq!(s1.status, SessionStatus::Unknown);
        assert_eq!(s1.pid, None);
        assert_eq!(s1.started_at, None);
        assert!(!s1.is_running());

        let s2 = &recent[1];
        assert_eq!(s2.last_active_at, 1_700_000_020_000, "mtime when history is silent");
        assert_eq!(s2.title.as_deref(), Some("Write docs"));

        let s3 = &recent[2];
        assert_eq!(s3.cwd, "/Users/adz/gone", "history-only session keeps its project");
        assert_eq!(s3.transcript_path, None);
        assert_eq!(s3.title, None);
    }

    #[test]
    fn limit_is_respected_and_zero_is_empty() {
        let f = Fixture::new();
        for i in 0..5 {
            f.simple_transcript("-Users-adz-p", &format!("id-{i}"), "hello", "/Users/adz/p");
        }
        let idx = f.index();
        assert_eq!(idx.recent_sessions(2).len(), 2);
        assert_eq!(idx.recent_sessions(50).len(), 5);
        assert!(idx.recent_sessions(0).is_empty());
    }

    #[test]
    fn enrich_fills_missing_fields_only() {
        let f = Fixture::new();
        let path = f.simple_transcript("-Users-adz-alpha", "id-1", "Refactor the parser", "/Users/adz/alpha");
        let idx = f.index();
        let running =
            Session { id: "id-1".into(), pid: Some(4), version: Some("9.9.9".into()), ..Default::default() };
        let s = idx.enrich(running);
        assert_eq!(s.title.as_deref(), Some("Refactor the parser"));
        assert_eq!(s.cwd, "/Users/adz/alpha");
        assert_eq!(s.git_branch.as_deref(), Some("main"));
        assert_eq!(s.version.as_deref(), Some("9.9.9"), "existing values are kept");
        assert_eq!(s.transcript_path.as_deref(), Some(path.to_str().unwrap()));
        assert_eq!(s.pid, Some(4));

        let unknown = idx.enrich(Session { id: "nope".into(), ..Default::default() });
        assert_eq!(unknown.title, None);
        assert_eq!(unknown.transcript_path, None);
        let empty = idx.enrich(Session::default());
        assert_eq!(empty.transcript_path, None);
    }

    #[test]
    fn title_skips_command_lines_and_uses_first_real_prompt() {
        let f = Fixture::new();
        let id = "id-t";
        let mut lines = preamble(id);
        lines.push(command_line(id, "/Users/adz"));
        lines.push(serde_json::json!({"type":"system","subtype":"local_command","content":"<command-name>/resume</command-name>"}).to_string());
        lines.push(serde_json::json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"x","content":"ok"}]},"cwd":"/Users/adz"}).to_string());
        lines.push(user_line("[Pasted text #1 +42 lines]   please   review\nthis", "/Users/adz", id));
        f.write_transcript("-Users-adz", id, &lines);
        let s = f.index().recent_sessions(1).remove(0);
        assert_eq!(s.title.as_deref(), Some("please review this"));
    }

    #[test]
    fn title_absent_when_no_prompt_in_head() {
        let f = Fixture::new();
        let id = "id-n";
        let mut lines = preamble(id);
        lines.push(command_line(id, "/Users/adz"));
        f.write_transcript("-Users-adz", id, &lines);
        let s = f.index().recent_sessions(1).remove(0);
        assert_eq!(s.title, None);
        assert_eq!(s.cwd, "/Users/adz", "cwd still comes from the command line");
    }

    #[test]
    fn subagent_transcripts_are_skipped() {
        let f = Fixture::new();
        f.simple_transcript("-Users-adz-p", "main-id", "main prompt", "/Users/adz/p");
        let sub_dir = f.projects().join("-Users-adz-p").join("main-id").join("subagents");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(
            sub_dir.join("agent-abc.jsonl"),
            user_line("sub prompt", "/Users/adz/p", "agent-abc") + "\n",
        )
        .unwrap();
        fs::write(
            sub_dir.join("11111111-2222-3333-4444-555555555555.jsonl"),
            user_line("sub", "/x", "s") + "\n",
        )
        .unwrap();
        // Legacy layout: agent transcripts next to the main one.
        f.simple_transcript("-Users-adz-p", "agent-legacy", "legacy agent", "/Users/adz/p");
        // Not a transcript at all.
        fs::write(f.projects().join("-Users-adz-p").join("notes.json"), "{}").unwrap();
        fs::write(f.projects().join("README.md"), "x").unwrap();
        let recent = f.index().recent_sessions(10);
        let ids: Vec<&str> = recent.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["main-id"]);
    }

    #[test]
    fn cache_hit_on_same_mtime_and_size_miss_after_touch() {
        let f = Fixture::new();
        let path = f.simple_transcript("-Users-adz-p", "id-c", "Title AAAA", "/Users/adz/p");
        let base = SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        f.set_mtime(&path, base);
        assert_eq!(f.index().recent_sessions(1)[0].title.as_deref(), Some("Title AAAA"));

        let cache_file = f.tmp.path().join("cache").join(CACHE_FILE);
        let cached = fs::read_to_string(&cache_file).unwrap();
        assert!(cached.contains("Title AAAA"));
        assert!(!cached.contains(SECRET_BODY), "cache must not hold message bodies");
        assert!(!cached.contains("command-name"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&cache_file).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(fs::metadata(f.tmp.path().join("cache")).unwrap().permissions().mode() & 0o777, 0o700);
        }
        let parsed: Value = serde_json::from_str(&cached).unwrap();
        let entry = &parsed["entries"][path.to_str().unwrap()];
        assert_eq!(entry["mtime_ms"], 1_700_000_000_000u64);
        assert_eq!(entry["size"], fs::metadata(&path).unwrap().len());
        assert_eq!(entry["cwd"], "/Users/adz/p");
        assert_eq!(entry["git_branch"], "main");
        assert_eq!(entry["version"], "2.1.260");

        // Same size, same mtime, different content: a fresh index trusts the cache.
        f.simple_transcript("-Users-adz-p", "id-c", "Title BBBB", "/Users/adz/p");
        f.set_mtime(&path, base);
        assert_eq!(f.index().recent_sessions(1)[0].title.as_deref(), Some("Title AAAA"), "cache hit");

        // Bump the mtime: re-read.
        f.set_mtime(&path, base + Duration::from_secs(5));
        assert_eq!(f.index().recent_sessions(1)[0].title.as_deref(), Some("Title BBBB"), "cache miss");

        // Corrupt cache file is ignored, not fatal.
        fs::write(&cache_file, "{not json").unwrap();
        assert_eq!(f.index().recent_sessions(1)[0].title.as_deref(), Some("Title BBBB"));
        assert!(
            !fs::read_dir(f.tmp.path().join("cache"))
                .unwrap()
                .flatten()
                .any(|e| e.path().extension().is_some_and(|x| x == "tmp")),
            "no temp files left behind"
        );
    }

    #[test]
    fn cache_is_capped_to_newest_entries() {
        let mut cache = HashMap::new();
        for i in 0..(CACHE_CAP + 25) as u64 {
            cache.insert(
                format!("/p/{i}.jsonl"),
                CacheEntry { mtime_ms: i, size: 1, title: None, cwd: None, git_branch: None, version: None },
            );
        }
        prune(&mut cache);
        assert_eq!(cache.len(), CACHE_CAP);
        assert!(!cache.contains_key("/p/0.jsonl"));
        assert!(cache.contains_key(&format!("/p/{}.jsonl", CACHE_CAP + 24)));
    }

    #[test]
    fn invalidate_and_new_files_are_picked_up() {
        let f = Fixture::new();
        f.simple_transcript("-Users-adz-p", "id-1", "one", "/Users/adz/p");
        let idx = f.index();
        assert_eq!(idx.recent_sessions(10).len(), 1);
        f.simple_transcript("-Users-adz-q", "id-2", "two", "/Users/adz/q");
        assert_eq!(idx.recent_sessions(10).len(), 2, "new project dir changes the signature");
        idx.invalidate();
        assert_eq!(idx.recent_sessions(10).len(), 2);
        let s = idx.enrich(Session { id: "id-2".into(), ..Default::default() });
        assert_eq!(s.title.as_deref(), Some("two"));
    }

    #[test]
    fn windows_style_cwd_is_parsed_unchanged() {
        let f = Fixture::new();
        let id = "id-w";
        let mut lines = preamble(id);
        lines.push(user_line("fix it", r"C:\Users\me\proj", id));
        f.write_transcript("C--Users-me-proj", id, &lines);
        f.write_history(&[history_line("id-w", 5, r"C:\Users\me\proj")]);
        let s = f.index().recent_sessions(1).remove(0);
        assert_eq!(s.cwd, r"C:\Users\me\proj");
        assert_eq!(s.project_name(), "proj");
    }

    #[test]
    fn history_only_keeps_newest_timestamp_and_project() {
        let f = Fixture::new();
        f.write_history(&[
            history_line("h", 10, "/a"),
            history_line("h", 30, "/c"),
            history_line("h", 20, "/b"),
            r#"{"sessionId":"h","timestamp":"40"}"#.to_string(),
        ]);
        let s = f.index().recent_sessions(5).remove(0);
        assert_eq!(s.last_active_at, 40);
        assert_eq!(s.cwd, "/c", "project of the newest line that had one");
    }

    #[test]
    fn head_only_reads_64kb() {
        let f = Fixture::new();
        let id = "id-big";
        let mut lines = preamble(id);
        // 70 KB of noise before the first prompt.
        for i in 0..700 {
            lines.push(format!(r#"{{"type":"attachment","i":{i},"pad":"{}"}}"#, "x".repeat(80)));
        }
        lines.push(user_line("late prompt", "/late", id));
        f.write_transcript("-Users-adz", id, &lines);
        let s = f.index().recent_sessions(1).remove(0);
        assert_eq!(s.title, None, "prompt beyond the head is never read");
    }

    #[test]
    fn missing_dirs_are_empty_not_panics() {
        let idx = TranscriptIndex::new(
            PathBuf::from("/definitely/not/here/projects"),
            PathBuf::from("/definitely/not/here/history.jsonl"),
            PathBuf::from("/definitely/not/here/cache"),
        );
        assert!(idx.recent_sessions(10).is_empty());
        let s = idx.enrich(Session { id: "x".into(), ..Default::default() });
        assert_eq!(s.transcript_path, None);
        idx.invalidate();
    }

    #[test]
    fn timing_300_fixtures() {
        let f = Fixture::new();
        for i in 0..300 {
            let enc = format!("-Users-adz-proj{}", i % 20);
            let id = format!("{:08x}-0000-4000-8000-{:012x}", i, i);
            f.simple_transcript(
                &enc,
                &id,
                &format!("Prompt number {i} with a bit of text"),
                &format!("/Users/adz/proj{}", i % 20),
            );
        }
        let mut history = Vec::new();
        for i in 0..300 {
            let id = format!("{:08x}-0000-4000-8000-{:012x}", i, i);
            history.push(history_line(&id, 1_700_000_000_000 + i as u64, "/Users/adz/x"));
        }
        f.write_history(&history);

        let cold = Instant::now();
        let first = f.index().recent_sessions(30);
        let cold = cold.elapsed();
        assert_eq!(first.len(), 30);
        assert!(first.iter().all(|s| s.title.is_some()));

        let idx = f.index(); // fresh memory, warm disk cache
        let warm_disk = Instant::now();
        let second = idx.recent_sessions(30);
        let warm_disk = warm_disk.elapsed();
        assert_eq!(first, second);

        let warm_mem = Instant::now();
        let third = idx.recent_sessions(30);
        let warm_mem = warm_mem.elapsed();
        assert_eq!(first, third);

        let all = Instant::now();
        let everything = idx.recent_sessions(300);
        let all = all.elapsed();
        assert_eq!(everything.len(), 300);

        println!(
            "recent_sessions over 300 transcripts: cold {cold:?}, warm disk cache {warm_disk:?}, warm memory {warm_mem:?}, limit=300 (all heads) {all:?}"
        );
        assert!(warm_mem < Duration::from_secs(2), "unexpectedly slow: {warm_mem:?}");
        let tmp = File::open(f.tmp.path().join("cache").join(CACHE_FILE)).unwrap();
        assert!(tmp.metadata().unwrap().len() > 0);
    }
}
