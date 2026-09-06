//! Single source of truth. Owns the core services, merges their output into a
//! `Snapshot`, and pushes it to every window (`snapshot` event) and the tray.

use crate::claude::paths::{home_dir, AppPaths, ClaudePaths};
use crate::claude::registry::Registry;
use crate::claude::transcripts::TranscriptIndex;
use crate::hooks::installer::HookInstaller;
use crate::hooks::notify;
use crate::hooks::watcher::HookWatcher;
use crate::launch::{self, LaunchError, LaunchRequest};
use crate::model::*;
use crate::settings::{Settings, SettingsStore};
use crate::{tray, windows};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const EVENT_LOG_CAP: usize = 200;

#[derive(Default)]
struct Inner {
    running: Vec<Session>,
    recent: Vec<Session>,
    attention: HashMap<String, HookEvent>,
    event_log: Vec<HookEvent>,
    hook_status: HookStatus,
    settings: Settings,
    last_error: Option<String>,
    is_refreshing_history: bool,
}

pub struct AppState {
    pub claude_paths: ClaudePaths,
    pub app_paths: AppPaths,
    pub registry: Registry,
    pub index: TranscriptIndex,
    pub installer: HookInstaller,
    pub settings_store: SettingsStore,
    inner: Mutex<Inner>,
    hook_watcher: Mutex<Option<HookWatcher>>,
    history_gen: AtomicU64,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Self {
        let data_dir = app.path().app_data_dir().unwrap_or_else(|_| home_dir().join(".heron"));
        let app_paths = AppPaths::new(data_dir);
        for dir in [
            app_paths.data_dir.clone(),
            app_paths.events_dir(),
            app_paths.cache_dir(),
            app_paths.launch_dir(),
        ] {
            if let Err(e) = create_private_dir(&dir) {
                log::warn!("could not create {}: {e}", dir.display());
            }
        }
        let claude_paths = ClaudePaths::detect();
        let settings_store = SettingsStore::new(&app_paths.data_dir);
        let settings = settings_store.load();
        let installer =
            HookInstaller::new(claude_paths.settings_file(), app_paths.hook_script(), app_paths.events_dir());
        let hook_status = installer.status();
        Self {
            registry: Registry::new(claude_paths.sessions_dir()),
            index: TranscriptIndex::new(
                claude_paths.projects_dir(),
                claude_paths.history_file(),
                app_paths.cache_dir(),
            ),
            installer,
            settings_store,
            claude_paths,
            app_paths,
            inner: Mutex::new(Inner { settings, hook_status, ..Default::default() }),
            hook_watcher: Mutex::new(None),
            history_gen: AtomicU64::new(0),
        }
    }

    // ---- reads -----------------------------------------------------------

    pub fn snapshot(&self) -> Snapshot {
        let inner = self.inner.lock();
        Snapshot {
            running: inner.running.clone(),
            recent: inner.recent.clone(),
            attention: inner.attention.clone(),
            event_log: inner.event_log.clone(),
            hook_status: inner.hook_status,
            claude_path: self.resolved_claude(&inner.settings).map(|p| p.display().to_string()),
            last_error: inner.last_error.clone(),
            is_refreshing_history: inner.is_refreshing_history,
            settings: inner.settings.clone(),
        }
    }

    pub fn settings(&self) -> Settings {
        self.inner.lock().settings.clone()
    }

    pub fn session(&self, id: &str) -> Option<Session> {
        let inner = self.inner.lock();
        inner.running.iter().chain(inner.recent.iter()).find(|s| s.id == id).cloned()
    }

    fn resolved_claude(&self, settings: &Settings) -> Option<PathBuf> {
        if !settings.claude_path.trim().is_empty() {
            return Some(PathBuf::from(settings.claude_path.trim()));
        }
        launch::find_claude()
    }

    fn terminal(&self, settings: &Settings) -> TerminalApp {
        settings.terminal_app.unwrap_or_else(launch::default_terminal)
    }

    // ---- publish ---------------------------------------------------------

    pub fn publish(&self, app: &AppHandle) {
        let snap = self.snapshot();
        tray::update(app, &snap);
        if let Err(e) = app.emit("snapshot", &snap) {
            log::warn!("emit snapshot failed: {e}");
        }
    }

    pub fn set_error(&self, app: &AppHandle, msg: impl Into<String>) {
        let msg = msg.into();
        log::warn!("{msg}");
        self.inner.lock().last_error = Some(msg);
        self.publish(app);
    }

    pub fn clear_error(&self, app: &AppHandle) {
        self.inner.lock().last_error = None;
        self.publish(app);
    }

    // ---- settings --------------------------------------------------------

    /// Merge a partial JSON object into the settings, persist, publish.
    pub fn update_settings(&self, app: &AppHandle, patch: serde_json::Value) -> Result<Settings, String> {
        let merged = {
            let inner = self.inner.lock();
            let mut current = serde_json::to_value(&inner.settings).map_err(|e| e.to_string())?;
            match (&mut current, patch) {
                (serde_json::Value::Object(cur), serde_json::Value::Object(p)) => {
                    for (k, v) in p {
                        cur.insert(k, v);
                    }
                }
                _ => return Err("settings patch must be an object".into()),
            }
            serde_json::from_value::<Settings>(current).map_err(|e| e.to_string())?
        };
        let previous = self.settings();
        self.settings_store.save(&merged).map_err(|e| e.to_string())?;
        self.inner.lock().settings = merged.clone();
        if previous.show_panel != merged.show_panel {
            windows::set_panel_visible(app, merged.show_panel);
        }
        if previous.panel_edge != merged.panel_edge
            || previous.panel_follows_active_screen != merged.panel_follows_active_screen
        {
            windows::place_panel(app);
        }
        if previous.recent_limit != merged.recent_limit {
            refresh_history(app);
        }
        self.publish(app);
        Ok(merged)
    }

    // ---- registry / hooks ------------------------------------------------

    fn apply_registry(&self, app: &AppHandle, mut sessions: Vec<Session>) {
        let changed = {
            let mut inner = self.inner.lock();
            sessions.sort_by_key(|s| std::cmp::Reverse(s.started_at));
            for s in sessions.iter_mut() {
                // Keep titles we already learned so rows do not flicker.
                if s.title.is_none() {
                    if let Some(old) = inner.running.iter().chain(inner.recent.iter()).find(|o| o.id == s.id)
                    {
                        s.title = old.title.clone();
                        s.git_branch = s.git_branch.clone().or_else(|| old.git_branch.clone());
                        s.transcript_path = s.transcript_path.clone().or_else(|| old.transcript_path.clone());
                    }
                }
                // A session that moved on since the attention event is no longer waiting.
                if let Some(a) = inner.attention.get(&s.id) {
                    if matches!(s.status, SessionStatus::Busy | SessionStatus::Idle)
                        && s.last_active_at > a.received_at
                    {
                        inner.attention.remove(&s.id);
                    }
                }
                s.attention = inner.attention.get(&s.id).cloned();
            }
            let changed = sessions != inner.running;
            inner.running = sessions;
            changed
        };
        if changed {
            self.publish(app);
            enrich_running(app);
            refresh_history_debounced(app);
        }
    }

    pub fn handle_event(&self, app: &AppHandle, event: HookEvent) {
        let (session, settings) = {
            let mut inner = self.inner.lock();
            inner.event_log.insert(0, event.clone());
            inner.event_log.truncate(EVENT_LOG_CAP);
            let session =
                inner.running.iter().chain(inner.recent.iter()).find(|s| s.id == event.session_id).cloned();
            if event.requires_attention() {
                inner.attention.insert(event.session_id.clone(), event.clone());
            } else if event.is_completion()
                || matches!(event.kind, HookEventKind::UserPromptSubmit | HookEventKind::SessionEnd)
            {
                inner.attention.remove(&event.session_id);
            }
            let attention = inner.attention.clone();
            for s in inner.running.iter_mut() {
                s.attention = attention.get(&s.id).cloned();
            }
            (session, inner.settings.clone())
        };
        let _ = app.emit("hook-event", &event);
        self.publish(app);

        let wants = if event.requires_attention() {
            settings.notify_on_permission
        } else if event.is_idle_prompt() {
            // Not an attention state, but the user can ask to hear about it.
            settings.notify_on_idle
        } else if event.is_completion() {
            settings.notify_on_done
        } else {
            false
        };
        if wants {
            notify::post(app, &event, session.as_ref(), &settings);
        }
        if matches!(event.kind, HookEventKind::SessionStart | HookEventKind::SessionEnd) {
            refresh_history_debounced(app);
        }
    }

    // ---- actions ---------------------------------------------------------

    pub fn new_session(&self, app: &AppHandle, folder: &Path) {
        let settings = self.settings();
        let Some(claude) = self.resolved_claude(&settings) else {
            self.set_error(app, LaunchError::ClaudeMissing.to_string());
            return;
        };
        let result = launch::launch(LaunchRequest {
            cwd: folder,
            terminal: self.terminal(&settings),
            claude: &claude,
            args: settings.extra_arg_list(),
            launch_dir: &self.app_paths.launch_dir(),
        });
        match result {
            Ok(()) => self.clear_error(app),
            Err(e) => self.set_error(app, e.to_string()),
        }
    }

    pub fn resume(&self, app: &AppHandle, id: &str) {
        let Some(session) = self.session(id) else {
            self.set_error(app, "that session is no longer listed");
            return;
        };
        if session.is_running() {
            self.focus(app, id);
            return;
        }
        let settings = self.settings();
        let Some(claude) = self.resolved_claude(&settings) else {
            self.set_error(app, LaunchError::ClaudeMissing.to_string());
            return;
        };
        let mut args = vec!["--resume".to_string(), session.id.clone()];
        args.extend(settings.extra_arg_list());
        let result = launch::launch(LaunchRequest {
            cwd: Path::new(&session.cwd),
            terminal: self.terminal(&settings),
            claude: &claude,
            args,
            launch_dir: &self.app_paths.launch_dir(),
        });
        match result {
            Ok(()) => self.clear_error(app),
            Err(e) => self.set_error(app, e.to_string()),
        }
    }

    pub fn focus(&self, app: &AppHandle, id: &str) -> bool {
        let Some(session) = self.session(id) else { return false };
        let settings = self.settings();
        let ok = launch::focus(&session, self.terminal(&settings));
        if !ok {
            log::info!("could not focus session {id}; terminal front-most fallback");
        }
        let _ = app;
        ok
    }

    pub fn terminate(&self, app: &AppHandle, id: &str) {
        if let Some(pid) = self.session(id).and_then(|s| s.pid) {
            if let Err(e) = launch::terminate(pid) {
                self.set_error(app, e.to_string());
            }
        }
    }

    pub fn install_hooks(&self, app: &AppHandle) -> Result<(), String> {
        let r = self.installer.install().map_err(|e| e.to_string());
        self.inner.lock().hook_status = self.installer.status();
        match &r {
            Ok(()) => {
                self.start_hook_watcher(app);
                self.clear_error(app);
            }
            Err(e) => self.set_error(app, e.clone()),
        }
        r
    }

    pub fn uninstall_hooks(&self, app: &AppHandle) -> Result<(), String> {
        let r = self.installer.uninstall().map_err(|e| e.to_string());
        self.inner.lock().hook_status = self.installer.status();
        match &r {
            Ok(()) => self.clear_error(app),
            Err(e) => self.set_error(app, e.clone()),
        }
        r
    }

    fn start_hook_watcher(&self, app: &AppHandle) {
        let mut slot = self.hook_watcher.lock();
        if slot.is_some() {
            return;
        }
        let handle = app.clone();
        match HookWatcher::start(self.app_paths.events_dir(), move |event| {
            let state = handle.state::<AppState>();
            state.handle_event(&handle, event);
        }) {
            Ok(w) => *slot = Some(w),
            Err(e) => log::warn!("hook watcher failed to start: {e}"),
        }
    }
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

// ---- background work --------------------------------------------------------

/// Start the registry poll and the hook watcher. Call once after `manage`.
pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.start_hook_watcher(app);

    let handle = app.clone();
    thread::Builder::new()
        .name("heron-registry".into())
        .spawn(move || loop {
            let state = handle.state::<AppState>();
            let sessions = state.registry.snapshot();
            state.apply_registry(&handle, sessions);
            thread::sleep(Duration::from_secs(1));
        })
        .expect("spawn registry thread");

    refresh_history(app);
}

/// Rescan history on a background thread; the newest request wins.
pub fn refresh_history(app: &AppHandle) {
    let state = app.state::<AppState>();
    let gen = state.history_gen.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let mut inner = state.inner.lock();
        inner.is_refreshing_history = true;
    }
    let handle = app.clone();
    thread::Builder::new()
        .name("heron-history".into())
        .spawn(move || {
            let state = handle.state::<AppState>();
            let limit = state.settings().recent_limit.max(1);
            let recent = state.index.recent_sessions(limit);
            if state.history_gen.load(Ordering::SeqCst) != gen {
                return; // superseded
            }
            {
                let mut inner = state.inner.lock();
                inner.recent = recent;
                inner.is_refreshing_history = false;
            }
            state.publish(&handle);
        })
        .expect("spawn history thread");
}

fn refresh_history_debounced(app: &AppHandle) {
    let handle = app.clone();
    let state = app.state::<AppState>();
    let gen = state.history_gen.load(Ordering::SeqCst);
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        let state = handle.state::<AppState>();
        if state.history_gen.load(Ordering::SeqCst) == gen {
            refresh_history(&handle);
        }
    });
}

/// Fill in titles for running sessions that do not have one yet.
fn enrich_running(app: &AppHandle) {
    let handle = app.clone();
    thread::spawn(move || {
        let state = handle.state::<AppState>();
        let missing: Vec<Session> =
            state.inner.lock().running.iter().filter(|s| s.title.is_none()).cloned().collect();
        if missing.is_empty() {
            return;
        }
        for s in missing {
            let enriched = state.index.enrich(s.clone());
            let mut inner = state.inner.lock();
            if let Some(r) = inner.running.iter_mut().find(|r| r.id == s.id) {
                r.title = enriched.title;
                r.git_branch = r.git_branch.clone().or(enriched.git_branch);
                r.transcript_path = r.transcript_path.clone().or(enriched.transcript_path);
            }
        }
        state.publish(&handle);
    });
}
