//! Registers Heron's hook receiver in `~/.claude/settings.json`.
//!
//! - `install` writes the receiver script (`bin/heron-hook` on Unix,
//!   `bin/heron-hook.ps1` on Windows; dir 0700, file 0700), creates the
//!   events dir (0700) and registers the script for `EVENTS` in the
//!   **exec form** (`command` + `args`, no shell): Unix `command` is the
//!   script path with `args: []`; Windows `command` is `powershell.exe` with
//!   `args: ["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass",
//!   "-File", <script>]`. Always `"async": true, "timeout": 5`; the
//!   `Notification` group carries `NOTIFICATION_MATCHER`.
//! - The script spools stdin (cap 256 KB) into `events_dir` as
//!   `<unix-seconds>-<pid>.json` (0600) through a temp file + rename, and
//!   always exits 0 with no output, so it can never block Claude Code.
//! - Heron's own groups are the ones whose hook `command` or any `args`
//!   element contains `heron-hook`. They are replaced in place; every other
//!   group, every other key and the key order survive (`serde_json` with
//!   `preserve_order`). A file that is not a JSON object is never touched.
//! - Before every write the file is copied to
//!   `settings.json.heron-backup-<yyyyMMdd-HHmmss>` (UTC; the 5 newest are
//!   kept), then replaced atomically (temp file + rename) keeping its mode.
//!   A re-install that would change nothing does not write or back up.
//! - `uninstall` removes exactly Heron's groups, drops event arrays and the
//!   `hooks` key only when that removal emptied them, deletes the script and
//!   an empty `bin/`, and leaves the events dir alone.
//! - `status` costs two small file reads and never panics.

use super::fsutil;
use crate::model::{HookEventKind, HookStatus};
use serde_json::{json, Map, Value};
use std::fs;
use std::io;
use std::path::PathBuf;

pub const EVENTS: [HookEventKind; 6] = [
    HookEventKind::SessionStart,
    HookEventKind::SessionEnd,
    HookEventKind::Stop,
    HookEventKind::Notification,
    HookEventKind::PermissionRequest,
    HookEventKind::UserPromptSubmit,
];

/// `Notification` subtypes Heron subscribes to (exact-list matcher syntax).
pub const NOTIFICATION_MATCHER: &str =
    "permission_prompt|idle_prompt|elicitation_dialog|elicitation_url_dialog|agent_needs_input|agent_completed";

/// Substring that identifies a hook as Heron's (it is the script's name).
const MARKER: &str = "heron-hook";
/// `settings.json` + this + `yyyyMMdd-HHmmss` = a backup file name.
const BACKUP_INFIX: &str = ".heron-backup-";
const BACKUPS_KEPT: usize = 5;
const HOOK_TIMEOUT_SECS: u64 = 5;
/// Bytes of stdin the receiver keeps; `watcher::MAX_EVENT_BYTES` is the same.
const STDIN_CAP: usize = 262_144;

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("~/.claude/settings.json is not a JSON object; not touching it")]
    NotAnObject,
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("the \"hooks\" section of ~/.claude/settings.json has an unexpected shape; not touching it")]
    HooksShape,
}

pub struct HookInstaller {
    pub settings_file: PathBuf,
    pub script_path: PathBuf,
    pub events_dir: PathBuf,
}

impl HookInstaller {
    pub fn new(settings_file: PathBuf, script_path: PathBuf, events_dir: PathBuf) -> Self {
        Self { settings_file, script_path, events_dir }
    }

    /// The receiver script for this platform.
    pub fn script_source(&self) -> String {
        if cfg!(windows) {
            self.powershell_script()
        } else {
            self.posix_script()
        }
    }

    pub fn status(&self) -> HookStatus {
        let script_current =
            fs::read(&self.script_path).is_ok_and(|cur| cur == self.script_source().as_bytes());
        let root = self.read_settings().unwrap_or_default();
        let hooks = root.get("hooks").and_then(Value::as_object);
        let hook = self.hook_object();
        let mut any = false;
        let mut current = 0;
        for kind in EVENTS {
            let heron: Vec<&Value> = hooks
                .and_then(|h| h.get(kind.name()))
                .and_then(Value::as_array)
                .map(|groups| groups.iter().filter(|g| is_heron_group(g)).collect())
                .unwrap_or_default();
            if heron.is_empty() {
                continue;
            }
            any = true;
            if heron.len() == 1 && *heron[0] == self.group_for(kind, &hook) {
                current += 1;
            }
        }
        if any && current == EVENTS.len() && script_current {
            HookStatus::Installed
        } else if any {
            HookStatus::Partial
        } else {
            HookStatus::NotInstalled
        }
    }

    pub fn install(&self) -> Result<(), InstallError> {
        fsutil::create_private_dir(&self.events_dir)?;
        self.write_script()?;
        let root = self.read_settings()?;
        let merged = self.merged(&root)?;
        if merged == root {
            log::debug!("hooks already registered; settings.json untouched");
            return Ok(());
        }
        self.write_settings(&merged)?;
        log::info!("registered Heron hooks in {}", self.settings_file.display());
        Ok(())
    }

    pub fn uninstall(&self) -> Result<(), InstallError> {
        let root = self.read_settings()?;
        let mut stripped = root.clone();
        if strip_heron_groups(&mut stripped) {
            self.write_settings(&stripped)?;
            log::info!("removed Heron hooks from {}", self.settings_file.display());
        }
        self.remove_script()?;
        Ok(())
    }

    /// Existing backups of `settings.json`, newest first.
    pub fn backups(&self) -> Vec<PathBuf> {
        let (Some(dir), Some(prefix)) = (self.settings_file.parent(), self.backup_prefix()) else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
        let mut found: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
            .map(|e| e.path())
            .collect();
        // Names start with a UTC timestamp, so name order is age order.
        found.sort();
        found.reverse();
        found
    }

    // ---- registration ----------------------------------------------------

    /// The hook object Heron registers on this platform.
    fn hook_object(&self) -> Value {
        self.hook_object_for(cfg!(windows))
    }

    fn hook_object_for(&self, windows: bool) -> Value {
        let script = self.script_path.to_string_lossy().into_owned();
        let (command, args) = if windows {
            (
                "powershell.exe".to_string(),
                json!(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script]),
            )
        } else {
            (script, json!([]))
        };
        json!({
            "type": "command",
            "command": command,
            "args": args,
            "async": true,
            "timeout": HOOK_TIMEOUT_SECS,
        })
    }

    /// The group (`matcher` + `hooks`) Heron registers for one event.
    fn group_for(&self, kind: HookEventKind, hook: &Value) -> Value {
        let mut group = Map::new();
        if kind == HookEventKind::Notification {
            group.insert("matcher".into(), Value::String(NOTIFICATION_MATCHER.into()));
        }
        group.insert("hooks".into(), Value::Array(vec![hook.clone()]));
        Value::Object(group)
    }

    /// `root` with Heron's groups inserted or refreshed for every event.
    fn merged(&self, root: &Map<String, Value>) -> Result<Map<String, Value>, InstallError> {
        let mut root = root.clone();
        let hook = self.hook_object();
        let Value::Object(hooks) = root.entry("hooks").or_insert_with(|| Value::Object(Map::new())) else {
            return Err(InstallError::HooksShape);
        };
        for kind in EVENTS {
            let group = self.group_for(kind, &hook);
            let Value::Array(groups) = hooks.entry(kind.name()).or_insert_with(|| Value::Array(Vec::new()))
            else {
                return Err(InstallError::HooksShape);
            };
            replace_heron_groups(groups, group);
        }
        Ok(root)
    }

    // ---- settings.json I/O -----------------------------------------------

    /// The parsed settings object; a missing or empty file counts as `{}`.
    fn read_settings(&self) -> Result<Map<String, Value>, InstallError> {
        let bytes = match fs::read(&self.settings_file) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Map::new()),
            Err(e) => return Err(e.into()),
        };
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Map::new());
        }
        match serde_json::from_slice::<Value>(&bytes)? {
            Value::Object(root) => Ok(root),
            _ => Err(InstallError::NotAnObject),
        }
    }

    /// Back up, then atomically replace `settings.json` with `root`
    /// pretty-printed (2 spaces, trailing newline), keeping its mode.
    fn write_settings(&self, root: &Map<String, Value>) -> Result<(), InstallError> {
        let mut bytes = serde_json::to_vec_pretty(root)?;
        bytes.push(b'\n');
        if let Some(parent) = self.settings_file.parent() {
            fs::create_dir_all(parent)?;
        }
        self.backup()?;
        let mode = fsutil::mode_of(&self.settings_file).unwrap_or(0o600);
        fsutil::atomic_write(&self.settings_file, &bytes, mode)?;
        Ok(())
    }

    fn backup_prefix(&self) -> Option<String> {
        self.settings_file.file_name().map(|n| format!("{}{BACKUP_INFIX}", n.to_string_lossy()))
    }

    /// Copy `settings.json` (if it exists) next to itself with a UTC
    /// timestamp, then prune to the newest `BACKUPS_KEPT`.
    fn backup(&self) -> io::Result<()> {
        if !self.settings_file.is_file() {
            return Ok(());
        }
        let (Some(dir), Some(prefix)) = (self.settings_file.parent(), self.backup_prefix()) else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "settings path has no file name"));
        };
        let base = format!("{prefix}{}", fsutil::utc_stamp(fsutil::now_secs()));
        let mut target = dir.join(&base);
        let mut n = 1;
        while target.exists() {
            target = dir.join(format!("{base}-{n}"));
            n += 1;
        }
        fs::copy(&self.settings_file, &target)?;
        for old in self.backups().into_iter().skip(BACKUPS_KEPT) {
            if let Err(e) = fs::remove_file(&old) {
                log::warn!("could not prune backup {}: {e}", old.display());
            }
        }
        Ok(())
    }

    // ---- receiver script -------------------------------------------------

    /// Write the script only when its content differs (atomic replace, so a
    /// hook Claude Code is executing right now keeps reading the old file).
    fn write_script(&self) -> io::Result<()> {
        let source = self.script_source();
        if fs::read(&self.script_path).is_ok_and(|cur| cur == source.as_bytes()) {
            fsutil::set_mode(&self.script_path, 0o700);
            return Ok(());
        }
        if let Some(dir) = self.script_path.parent() {
            fsutil::create_private_dir(dir)?;
        }
        fsutil::atomic_write(&self.script_path, source.as_bytes(), 0o700)
    }

    fn remove_script(&self) -> io::Result<()> {
        match fs::remove_file(&self.script_path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        if let Some(dir) = self.script_path.parent() {
            // Only succeeds when nothing else lives in bin/.
            let _ = fs::remove_dir(dir);
        }
        Ok(())
    }

    fn posix_script(&self) -> String {
        let dir = fsutil::sh_quote(&self.events_dir.to_string_lossy());
        format!(
            "#!/bin/sh
# heron-hook -- Claude Code hook receiver, installed by Heron.
#
# Heron (the local Claude Code session tray app) registered this file in
# ~/.claude/settings.json. Claude Code runs it on a few events with the
# event JSON on stdin; the script only copies that JSON into Heron's own
# events folder so Heron can show a notification. No network, no output,
# never blocks Claude Code: it always exits 0.
#
# Deleting this file only disables Heron's notifications. Heron's
# Settings -> Hooks reinstalls or removes it and its settings.json entries.
umask 077
exec >/dev/null 2>&1
d={dir}
mkdir -p \"$d\"
t=\"$d/.$$.tmp\"
head -c {STDIN_CAP} >\"$t\"
mv -f \"$t\" \"$d/$(date +%s)-$$.json\"
exit 0
"
        )
    }

    fn powershell_script(&self) -> String {
        let dir = fsutil::ps_quote(&self.events_dir.to_string_lossy());
        format!(
            "# heron-hook.ps1 -- Claude Code hook receiver, installed by Heron.
#
# Heron (the local Claude Code session tray app) registered this file in
# %USERPROFILE%\\.claude\\settings.json. Claude Code runs it on a few events
# with the event JSON on stdin; the script only copies that JSON into
# Heron's own events folder so Heron can show a notification. No network,
# no output, never blocks Claude Code: it always exits 0.
#
# Deleting this file only disables Heron's notifications. Heron's
# Settings -> Hooks reinstalls or removes it and its settings.json entries.
$ErrorActionPreference = 'SilentlyContinue'
try {{
  $d = {dir}
  New-Item -ItemType Directory -Force -Path $d | Out-Null
  $m = New-Object System.IO.MemoryStream
  [Console]::OpenStandardInput().CopyTo($m)
  [byte[]]$b = $m.ToArray()
  if ($b.Length -gt {STDIN_CAP}) {{ [Array]::Resize([ref]$b, {STDIN_CAP}) }}
  $t = Join-Path $d \".$PID.tmp\"
  [System.IO.File]::WriteAllBytes($t, $b)
  $n = Join-Path $d ('{{0}}-{{1}}.json' -f [DateTimeOffset]::UtcNow.ToUnixTimeSeconds(), $PID)
  Move-Item -Force -LiteralPath $t -Destination $n
}} catch {{}}
exit 0
"
        )
    }
}

// ---- pure helpers -----------------------------------------------------------

fn is_heron_hook(hook: &Value) -> bool {
    let in_command = hook.get("command").and_then(Value::as_str).is_some_and(|c| c.contains(MARKER));
    let in_args = hook
        .get("args")
        .and_then(Value::as_array)
        .is_some_and(|args| args.iter().any(|a| a.as_str().is_some_and(|s| s.contains(MARKER))));
    in_command || in_args
}

fn is_heron_group(group: &Value) -> bool {
    group.get("hooks").and_then(Value::as_array).is_some_and(|hooks| hooks.iter().any(is_heron_hook))
}

/// Replace the first Heron group in `groups` with `replacement`, drop any
/// further ones, or append when there is none. Foreign groups keep their
/// positions.
fn replace_heron_groups(groups: &mut Vec<Value>, replacement: Value) {
    let mut placed = false;
    groups.retain_mut(|group| {
        if !is_heron_group(group) {
            return true;
        }
        if placed {
            return false;
        }
        *group = replacement.clone();
        placed = true;
        true
    });
    if !placed {
        groups.push(replacement);
    }
}

/// Remove every Heron group under `hooks`. Event arrays emptied by that
/// removal disappear, and so does `hooks` itself when nothing is left in
/// it. Returns whether anything was removed.
fn strip_heron_groups(root: &mut Map<String, Value>) -> bool {
    let Some(Value::Object(hooks)) = root.get_mut("hooks") else { return false };
    let mut removed = false;
    let mut emptied = Vec::new();
    for (name, entry) in hooks.iter_mut() {
        let Value::Array(groups) = entry else { continue };
        let before = groups.len();
        groups.retain(|g| !is_heron_group(g));
        if groups.len() != before {
            removed = true;
            if groups.is_empty() {
                emptied.push(name.clone());
            }
        }
    }
    for name in emptied {
        hooks.shift_remove(&name);
    }
    if removed && hooks.is_empty() {
        root.shift_remove("hooks");
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    const FOREIGN: &str = r#"{
  "model": "opus",
  "permissions": { "allow": ["Bash(ls:*)"] },
  "hooks": {
    "PreToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "echo pre" }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "command", "command": "say done", "timeout": 10 }] }
    ]
  },
  "env": { "FOO": "bar" }
}
"#;

    struct Fixture {
        _dir: TempDir,
        installer: HookInstaller,
    }

    fn fixture(settings: Option<&str>) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let claude = dir.path().join("claude");
        fs::create_dir_all(&claude).unwrap();
        let settings_file = claude.join("settings.json");
        if let Some(text) = settings {
            fs::write(&settings_file, text).unwrap();
        }
        let app = dir.path().join("app data");
        let script = app.join("bin").join(if cfg!(windows) { "heron-hook.ps1" } else { "heron-hook" });
        let installer = HookInstaller::new(settings_file, script, app.join("events"));
        Fixture { _dir: dir, installer }
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    fn without_heron(mut root: Map<String, Value>) -> Map<String, Value> {
        strip_heron_groups(&mut root);
        root
    }

    #[test]
    fn install_into_missing_settings_creates_six_groups() {
        let f = fixture(None);
        let i = &f.installer;
        i.install().unwrap();

        let root = read_json(&i.settings_file);
        let hooks = root["hooks"].as_object().unwrap();
        let names: Vec<&String> = hooks.keys().collect();
        assert_eq!(
            names,
            ["SessionStart", "SessionEnd", "Stop", "Notification", "PermissionRequest", "UserPromptSubmit"]
        );
        let expected_hook = i.hook_object();
        for kind in EVENTS {
            let groups = hooks[kind.name()].as_array().unwrap();
            assert_eq!(groups.len(), 1, "{}", kind.name());
            assert_eq!(groups[0]["hooks"], json!([expected_hook]));
            if kind == HookEventKind::Notification {
                assert_eq!(groups[0]["matcher"], NOTIFICATION_MATCHER);
            } else {
                assert!(groups[0].get("matcher").is_none());
            }
        }
        assert_eq!(expected_hook["async"], true);
        assert_eq!(expected_hook["timeout"], 5);
        assert!(i.backups().is_empty(), "nothing to back up on a fresh file");
        assert_eq!(fs::read_to_string(&i.script_path).unwrap(), i.script_source());
        assert!(i.events_dir.is_dir());
        #[cfg(unix)]
        {
            assert_eq!(fsutil::mode_of(&i.script_path), Some(0o700));
            assert_eq!(fsutil::mode_of(i.script_path.parent().unwrap()), Some(0o700));
            assert_eq!(fsutil::mode_of(&i.events_dir), Some(0o700));
            assert_eq!(fsutil::mode_of(&i.settings_file), Some(0o600));
        }
        assert_eq!(i.status(), HookStatus::Installed);
    }

    #[test]
    fn hook_shapes_per_platform() {
        let f = fixture(None);
        let script = f.installer.script_path.to_string_lossy().into_owned();
        let unix = f.installer.hook_object_for(false);
        assert_eq!(unix["command"], script);
        assert_eq!(unix["args"], json!([]));
        let win = f.installer.hook_object_for(true);
        assert_eq!(win["command"], "powershell.exe");
        assert_eq!(
            win["args"],
            json!(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script])
        );
        for hook in [&unix, &win] {
            assert!(is_heron_hook(hook));
            let keys: Vec<&String> = hook.as_object().unwrap().keys().collect();
            assert_eq!(keys, ["type", "command", "args", "async", "timeout"]);
        }
    }

    #[test]
    fn install_preserves_foreign_hooks_and_key_order() {
        let f = fixture(Some(FOREIGN));
        let i = &f.installer;
        i.install().unwrap();

        let text = fs::read_to_string(&i.settings_file).unwrap();
        let root: Map<String, Value> = serde_json::from_str(&text).unwrap();
        let top: Vec<&String> = root.keys().collect();
        assert_eq!(top, ["model", "permissions", "hooks", "env"]);
        let hooks = root["hooks"].as_object().unwrap();
        let names: Vec<&String> = hooks.keys().collect();
        assert_eq!(
            names,
            [
                "PreToolUse",
                "Stop",
                "SessionStart",
                "SessionEnd",
                "Notification",
                "PermissionRequest",
                "UserPromptSubmit"
            ]
        );
        let stop = hooks["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
        assert_eq!(stop[0]["hooks"][0]["command"], "say done", "foreign group stays first");
        assert!(is_heron_group(&stop[1]));

        let original: Map<String, Value> = serde_json::from_str(FOREIGN).unwrap();
        assert_eq!(without_heron(root), original, "everything that is not Heron's is unchanged");
        assert!(text.starts_with("{\n  \"model\""), "two-space pretty print");
        assert!(text.ends_with("}\n"));
        assert_eq!(i.backups().len(), 1);
        assert_eq!(fs::read_to_string(&i.backups()[0]).unwrap(), FOREIGN);
    }

    #[test]
    fn reinstall_is_idempotent() {
        let f = fixture(Some(FOREIGN));
        let i = &f.installer;
        i.install().unwrap();
        let first = fs::read(&i.settings_file).unwrap();
        let first_script_mtime = fs::metadata(&i.script_path).unwrap().modified().unwrap();
        i.install().unwrap();
        assert_eq!(fs::read(&i.settings_file).unwrap(), first, "byte-identical");
        assert_eq!(i.backups().len(), 1, "no second backup");
        assert_eq!(fs::metadata(&i.script_path).unwrap().modified().unwrap(), first_script_mtime);
        assert_eq!(i.status(), HookStatus::Installed);
    }

    #[test]
    fn install_refreshes_stale_heron_groups_in_place() {
        let stale = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"/old/heron-hook"}]},{"hooks":[{"type":"command","command":"other"}]}],"Notification":[{"matcher":"idle_prompt","hooks":[{"type":"command","command":"/old/heron-hook"}]},{"hooks":[{"type":"command","command":"/old/bin/heron-hook","args":[]}]}]}}"#;
        let f = fixture(Some(stale));
        let i = &f.installer;
        assert_eq!(i.status(), HookStatus::Partial);
        i.install().unwrap();
        let root = read_json(&i.settings_file);
        let stop = root["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
        assert_eq!(stop[0], i.group_for(HookEventKind::Stop, &i.hook_object()), "replaced in place");
        assert_eq!(stop[1]["hooks"][0]["command"], "other");
        let notification = root["hooks"]["Notification"].as_array().unwrap();
        assert_eq!(notification.len(), 1, "duplicate Heron groups collapse");
        assert_eq!(notification[0]["matcher"], NOTIFICATION_MATCHER);
        assert_eq!(i.status(), HookStatus::Installed);
    }

    #[test]
    fn uninstall_restores_original_hooks_exactly() {
        let f = fixture(Some(FOREIGN));
        let i = &f.installer;
        i.install().unwrap();
        i.uninstall().unwrap();
        let original: Value = serde_json::from_str(FOREIGN).unwrap();
        assert_eq!(read_json(&i.settings_file), original);
        assert!(!i.script_path.exists());
        assert!(!i.script_path.parent().unwrap().exists(), "empty bin/ removed");
        assert!(i.events_dir.is_dir(), "events dir left alone");
        assert_eq!(i.backups().len(), 2);
        assert_eq!(i.status(), HookStatus::NotInstalled);
    }

    #[test]
    fn uninstall_drops_emptied_arrays_and_hooks_key() {
        let f = fixture(None);
        let i = &f.installer;
        i.install().unwrap();
        i.uninstall().unwrap();
        assert_eq!(read_json(&i.settings_file), json!({}));
        // A second uninstall changes nothing and makes no backup.
        let backups = i.backups().len();
        i.uninstall().unwrap();
        assert_eq!(i.backups().len(), backups);
    }

    #[test]
    fn uninstall_keeps_foreign_empty_arrays() {
        let f = fixture(Some(r#"{"hooks":{"PreToolUse":[]},"other":1}"#));
        let i = &f.installer;
        i.install().unwrap();
        i.uninstall().unwrap();
        assert_eq!(read_json(&i.settings_file), json!({"hooks":{"PreToolUse":[]},"other":1}));
    }

    #[test]
    fn status_transitions() {
        let f = fixture(Some(FOREIGN));
        let i = &f.installer;
        assert_eq!(i.status(), HookStatus::NotInstalled);
        i.install().unwrap();
        assert_eq!(i.status(), HookStatus::Installed);

        fs::write(&i.script_path, "#!/bin/sh\nexit 0\n").unwrap();
        assert_eq!(i.status(), HookStatus::Partial, "stale script");
        fs::remove_file(&i.script_path).unwrap();
        assert_eq!(i.status(), HookStatus::Partial, "missing script");
        i.install().unwrap();
        assert_eq!(i.status(), HookStatus::Installed);

        let mut root: Map<String, Value> = read_json(&i.settings_file).as_object().unwrap().clone();
        root["hooks"].as_object_mut().unwrap().shift_remove("SessionEnd");
        fs::write(&i.settings_file, serde_json::to_vec(&root).unwrap()).unwrap();
        assert_eq!(i.status(), HookStatus::Partial, "one event missing");

        i.uninstall().unwrap();
        assert_eq!(i.status(), HookStatus::NotInstalled);
    }

    #[test]
    fn status_survives_bad_files() {
        let f = fixture(Some("this is not json"));
        assert_eq!(f.installer.status(), HookStatus::NotInstalled);
        assert!(matches!(f.installer.install(), Err(InstallError::Json(_))));
        let f = fixture(Some("[1, 2]"));
        assert_eq!(f.installer.status(), HookStatus::NotInstalled);
        assert!(matches!(f.installer.install(), Err(InstallError::NotAnObject)));
        let f = fixture(Some(r#"{"hooks": "nope"}"#));
        assert!(matches!(f.installer.install(), Err(InstallError::HooksShape)));
        assert_eq!(fs::read_to_string(&f.installer.settings_file).unwrap(), r#"{"hooks": "nope"}"#);
        let f = fixture(Some("  \n"));
        f.installer.install().unwrap();
        assert_eq!(f.installer.status(), HookStatus::Installed);
    }

    #[test]
    fn backup_rotation_keeps_five_newest() {
        let f = fixture(Some(FOREIGN));
        let i = &f.installer;
        i.install().unwrap();
        for _ in 0..6 {
            i.uninstall().unwrap();
            i.install().unwrap();
        }
        let kept = i.backups();
        assert_eq!(kept.len(), 5);
        let dir = i.settings_file.parent().unwrap();
        let all: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains(BACKUP_INFIX))
            .collect();
        assert_eq!(all.len(), 5, "older backups are gone");
        assert!(kept.windows(2).all(|w| w[0] > w[1]), "newest first");
        let name = kept[0].file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("settings.json.heron-backup-20"), "{name}");
    }

    #[test]
    fn scripts_embed_the_events_dir_safely() {
        let dir = tempfile::tempdir().unwrap();
        let events = dir.path().join("it's here").join("events");
        let i = HookInstaller::new(dir.path().join("s.json"), dir.path().join("heron-hook"), events.clone());
        let sh = i.posix_script();
        assert!(sh.starts_with("#!/bin/sh\n"));
        assert!(sh.lines().count() < 30);
        assert!(sh.contains(&format!("d={}", fsutil::sh_quote(&events.to_string_lossy()))));
        assert!(sh.contains("umask 077"));
        assert!(sh.contains("head -c 262144"));
        assert!(sh.trim_end().ends_with("exit 0"));
        let ps = i.powershell_script();
        assert!(ps.lines().count() < 30);
        assert!(ps.contains(&format!("$d = {}", fsutil::ps_quote(&events.to_string_lossy()))));
        assert!(ps.contains("$ErrorActionPreference = 'SilentlyContinue'"));
        assert!(ps.contains("[Console]::OpenStandardInput()"));
        assert!(ps.trim_end().ends_with("exit 0"));
    }

    /// The generated Unix script, run for real: exit 0, silent, spools stdin
    /// into a 0600 file named `<seconds>-<pid>.json`.
    #[cfg(unix)]
    #[test]
    fn unix_script_spools_stdin() {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let f = fixture(None);
        let i = &f.installer;
        i.install().unwrap();
        // The dir is created by the script itself when missing.
        fs::remove_dir(&i.events_dir).unwrap();

        let run = |payload: &[u8]| {
            let mut child = Command::new("/bin/sh")
                .arg(&i.script_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(payload).unwrap();
            child.wait_with_output().unwrap()
        };
        let payload = br#"{"hook_event_name":"Stop","session_id":"abc","cwd":"/tmp/x"}"#;
        let out = run(payload);
        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty(), "stdout must stay empty");
        assert!(out.stderr.is_empty(), "stderr must stay empty");

        let files: Vec<PathBuf> = fs::read_dir(&i.events_dir).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(files.len(), 1, "one spool file, no leftover temp: {files:?}");
        let name = files[0].file_name().unwrap().to_string_lossy().into_owned();
        let (secs, rest) = name.split_once('-').unwrap();
        assert!(secs.len() >= 10 && secs.bytes().all(|b| b.is_ascii_digit()), "{name}");
        assert!(rest.strip_suffix(".json").unwrap().bytes().all(|b| b.is_ascii_digit()), "{name}");
        assert_eq!(fs::read(&files[0]).unwrap(), payload);
        assert_eq!(fsutil::mode_of(&files[0]), Some(0o600));
        assert_eq!(fsutil::mode_of(&i.events_dir), Some(0o700));
        fs::remove_file(&files[0]).unwrap();

        let big = vec![b'x'; STDIN_CAP + 10_000];
        let out = run(&big);
        assert_eq!(out.status.code(), Some(0));
        let files: Vec<PathBuf> = fs::read_dir(&i.events_dir).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(files.len(), 1);
        assert_eq!(fs::metadata(&files[0]).unwrap().len(), STDIN_CAP as u64, "stdin capped at 256 KB");
    }
}
