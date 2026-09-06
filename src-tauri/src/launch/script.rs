//! One-shot launch scripts. Pure rendering plus a private-permission writer,
//! so the exact bytes can be unit-tested on every OS.
//!
//! - Unix (`.command` on macOS, `.sh` on Linux): `rm -f "$0"` first (bash
//!   keeps reading the unlinked file), widen `PATH` for GUI launches, `cd`
//!   with a readable failure, then `exec` claude so the tty belongs to it
//!   (which is what `focus` relies on). Every value is `shell_quote`d.
//! - Windows (`.cmd`): batch has no reliable quoting, so cwd, executable and
//!   arguments are *validated* — any of `" % ! & | < > ^` or a control
//!   character is refused with `LaunchError::Other` — and then wrapped in
//!   double quotes. The script deletes itself on its last line with the
//!   `(goto) 2>nul & del` idiom.
//! - Files are `heron-<random>.<ext>`, created with `create_new` (never
//!   clobbering, never following a symlink), 0700 on Unix in a 0700 dir.

use super::{shell_quote, LaunchError};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Which script dialect to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    MacosCommand,
    LinuxSh,
    WindowsCmd,
}

impl Flavor {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacosCommand
        } else if cfg!(windows) {
            Self::WindowsCmd
        } else {
            Self::LinuxSh
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::MacosCommand => "command",
            Self::LinuxSh => "sh",
            Self::WindowsCmd => "cmd",
        }
    }
}

/// What the script must do: `cd cwd && exec exe args...`.
pub struct Spec<'a> {
    pub cwd: &'a Path,
    pub exe: &'a Path,
    pub args: &'a [String],
}

/// Script text for `flavor`.
pub fn render(spec: &Spec<'_>, flavor: Flavor) -> Result<String, LaunchError> {
    match flavor {
        Flavor::MacosCommand | Flavor::LinuxSh => unix_script(spec),
        Flavor::WindowsCmd => windows_script(spec),
    }
}

/// bash script; every value single-quoted.
pub fn unix_script(spec: &Spec<'_>) -> Result<String, LaunchError> {
    let cwd = shell_quote(utf8(spec.cwd, "folder")?);
    let exe = shell_quote(utf8(spec.exe, "claude path")?);
    let mut s = String::new();
    s.push_str("#!/bin/bash\n");
    s.push_str("# Heron: starts a Claude Code session. Safe to delete.\n");
    s.push_str("rm -f -- \"$0\"\n");
    s.push_str("export PATH=\"$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"\n");
    s.push_str(&format!("cd {cwd} || {{ echo \"Heron: folder not found:\" {cwd}; read -r; exit 1; }}\n"));
    s.push_str(&format!("exec {exe}"));
    for a in spec.args {
        s.push(' ');
        s.push_str(&shell_quote(a));
    }
    s.push('\n');
    Ok(s)
}

/// Characters batch would interpret even inside double quotes, or that
/// would break the line structure. Refused rather than escaped.
const CMD_FORBIDDEN: &[char] = &['"', '%', '!', '&', '|', '<', '>', '^'];

fn check_cmd_value(value: &str, what: &str) -> Result<(), LaunchError> {
    if let Some(c) = value.chars().find(|c| CMD_FORBIDDEN.contains(c) || c.is_control()) {
        let shown = if c.is_control() { format!("U+{:04X}", c as u32) } else { c.to_string() };
        return Err(LaunchError::Other(format!(
            "the {what} contains a character ({shown}) that cannot be launched on Windows"
        )));
    }
    Ok(())
}

/// `.cmd` batch file with CRLF line endings; every value validated then
/// double-quoted.
pub fn windows_script(spec: &Spec<'_>) -> Result<String, LaunchError> {
    let cwd = utf8(spec.cwd, "folder")?;
    let exe = utf8(spec.exe, "claude path")?;
    check_cmd_value(cwd, "folder")?;
    check_cmd_value(exe, "claude path")?;
    for a in spec.args {
        check_cmd_value(a, "argument")?;
    }
    let mut s = String::new();
    s.push_str("@echo off\r\n");
    s.push_str("rem Heron: starts a Claude Code session. Safe to delete.\r\n");
    s.push_str(&format!(
        "cd /d \"{cwd}\" || (echo Heron: folder not found: \"{cwd}\" & pause & exit /b 1)\r\n"
    ));
    s.push_str(&format!("\"{exe}\""));
    for a in spec.args {
        s.push_str(&format!(" \"{a}\""));
    }
    s.push_str("\r\n");
    s.push_str("(goto) 2>nul & del \"%~f0\"\r\n");
    Ok(s)
}

/// Create the script in `launch_dir` (0700 dir, 0700 file on Unix) and
/// return its path.
pub fn write(launch_dir: &Path, spec: &Spec<'_>, flavor: Flavor) -> Result<PathBuf, LaunchError> {
    let content = render(spec, flavor)?;
    ensure_private_dir(launch_dir)?;
    for _ in 0..16 {
        let path = launch_dir.join(format!("heron-{}.{}", random_token(), flavor.extension()));
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o700);
        }
        match opts.open(&path) {
            Ok(mut file) => {
                let written = file.write_all(content.as_bytes()).and_then(|_| file.sync_all());
                drop(file);
                if let Err(e) = written {
                    let _ = fs::remove_file(&path);
                    return Err(e.into());
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
                }
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(LaunchError::Other("could not create a unique launch script".into()))
}

/// Remove leftover `heron-*` scripts older than an hour (a launch that never
/// ran, or a Windows session that was killed before its self-delete line).
pub fn sweep_stale(launch_dir: &Path) {
    let Ok(entries) = fs::read_dir(launch_dir) else { return };
    let cutoff = Duration::from_secs(60 * 60);
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let ours = name.starts_with("heron-")
            && path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "command" | "sh" | "cmd"));
        if !ours {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| m.elapsed().ok())
            .is_some_and(|age| age > cutoff);
        if old {
            let _ = fs::remove_file(&path);
        }
    }
}

fn ensure_private_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn utf8<'a>(path: &'a Path, what: &str) -> Result<&'a str, LaunchError> {
    path.to_str().ok_or_else(|| LaunchError::Other(format!("the {what} is not valid UTF-8")))
}

/// 12 hex digits from a randomly seeded hasher over the clock and pid. Not a
/// secret — uniqueness is enforced by `create_new` — just unguessable enough
/// that nothing can pre-create the name.
fn random_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.write_u32(std::process::id());
    format!("{:012x}", h.finish() & 0xffff_ffff_ffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn unix_script_quotes_space_and_apostrophe() {
        let a = args(&["--resume", "4e035c9a-1ab6-4026-aed1-0fa8731e7792", "--model", "opus"]);
        let spec = Spec {
            cwd: Path::new("/Users/you/My Projects/it's here"),
            exe: Path::new("/Users/you/.local/bin/claude"),
            args: &a,
        };
        let s = unix_script(&spec).unwrap();
        let expected = "#!/bin/bash\n\
            # Heron: starts a Claude Code session. Safe to delete.\n\
            rm -f -- \"$0\"\n\
            export PATH=\"$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"\n\
            cd '/Users/you/My Projects/it'\\''s here' || { echo \"Heron: folder not found:\" '/Users/you/My Projects/it'\\''s here'; read -r; exit 1; }\n\
            exec '/Users/you/.local/bin/claude' '--resume' '4e035c9a-1ab6-4026-aed1-0fa8731e7792' '--model' 'opus'\n";
        assert_eq!(s, expected);
    }

    #[test]
    fn unix_script_neutralises_shell_metacharacters() {
        let a = args(&["$(touch /tmp/pwned)", "`id`", "a\nb"]);
        let spec = Spec { cwd: Path::new("/tmp/x; rm -rf ~"), exe: Path::new("claude"), args: &a };
        let s = unix_script(&spec).unwrap();
        assert!(s.contains("cd '/tmp/x; rm -rf ~' ||"));
        assert!(s.ends_with("exec 'claude' '$(touch /tmp/pwned)' '`id`' 'a\nb'\n"));
        // Nothing outside single quotes but the fixed template.
        assert_eq!(s.matches("exec ").count(), 1);
    }

    #[test]
    fn windows_script_for_a_normal_path() {
        let a = args(&["--resume", "4e035c9a-1ab6-4026-aed1-0fa8731e7792"]);
        let spec = Spec {
            cwd: Path::new(r"C:\Users\me\Projects\app"),
            exe: Path::new(r"C:\Users\me\.local\bin\claude.exe"),
            args: &a,
        };
        let s = windows_script(&spec).unwrap();
        let expected = "@echo off\r\n\
            rem Heron: starts a Claude Code session. Safe to delete.\r\n\
            cd /d \"C:\\Users\\me\\Projects\\app\" || (echo Heron: folder not found: \"C:\\Users\\me\\Projects\\app\" & pause & exit /b 1)\r\n\
            \"C:\\Users\\me\\.local\\bin\\claude.exe\" \"--resume\" \"4e035c9a-1ab6-4026-aed1-0fa8731e7792\"\r\n\
            (goto) 2>nul & del \"%~f0\"\r\n";
        assert_eq!(s, expected);
    }

    #[test]
    fn windows_script_allows_spaces_and_parentheses() {
        let a = args(&[]);
        let spec = Spec {
            cwd: Path::new(r"C:\Program Files (x86)\Some App"),
            exe: Path::new(r"C:\Users\John Doe\AppData\Roaming\npm\claude.cmd"),
            args: &a,
        };
        let s = windows_script(&spec).unwrap();
        assert!(s.contains("cd /d \"C:\\Program Files (x86)\\Some App\" ||"));
        assert!(s.contains("\"C:\\Users\\John Doe\\AppData\\Roaming\\npm\\claude.cmd\"\r\n"));
    }

    #[test]
    fn windows_script_rejects_batch_metacharacters() {
        let exe = Path::new(r"C:\claude.exe");
        for bad in [
            r"C:\x\%TEMP%",
            "C:\\x\\say \"hi\"",
            r"C:\x\a!b",
            r"C:\x\a&b",
            r"C:\x\a|b",
            r"C:\x\a<b",
            r"C:\x\a>b",
            r"C:\x\a^b",
            "C:\\x\\a\r\nb",
            "C:\\x\\a\nb",
        ] {
            let a = args(&[]);
            let spec = Spec { cwd: Path::new(bad), exe, args: &a };
            let err = windows_script(&spec).unwrap_err();
            assert!(matches!(err, LaunchError::Other(_)), "{bad:?}: {err}");
            assert!(err.to_string().contains("folder"), "{bad:?}: {err}");
        }
        let a = args(&["--flag=%PATH%"]);
        let spec = Spec { cwd: Path::new(r"C:\x"), exe, args: &a };
        let err = windows_script(&spec).unwrap_err();
        assert!(err.to_string().contains("argument"), "{err}");
        let a = args(&[]);
        let spec = Spec { cwd: Path::new(r"C:\x"), exe: Path::new("C:\\claude \"x\".exe"), args: &a };
        let err = windows_script(&spec).unwrap_err();
        assert!(err.to_string().contains("claude path"), "{err}");
    }

    #[test]
    fn write_creates_private_script_in_private_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let launch_dir = tmp.path().join("launch");
        let a = args(&["--resume", "abc"]);
        let spec = Spec { cwd: tmp.path(), exe: Path::new("/bin/echo"), args: &a };
        let path = write(&launch_dir, &spec, Flavor::MacosCommand).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("heron-") && name.ends_with(".command"), "{name}");
        assert_eq!(name.len(), "heron-".len() + 12 + ".command".len());
        assert_eq!(fs::read_to_string(&path).unwrap(), render(&spec, Flavor::MacosCommand).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(fs::metadata(&launch_dir).unwrap().permissions().mode() & 0o777, 0o700);
        }
        let second = write(&launch_dir, &spec, Flavor::LinuxSh).unwrap();
        assert_ne!(path, second);
        assert!(second.extension().unwrap() == "sh");
        let third = write(&launch_dir, &spec, Flavor::WindowsCmd).unwrap();
        assert!(third.extension().unwrap() == "cmd");
        assert!(fs::read(&third).unwrap().starts_with(b"@echo off\r\n"));
    }

    #[test]
    fn sweep_removes_only_old_heron_scripts() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let old = dir.join("heron-000000000001.command");
        let fresh = dir.join("heron-000000000002.command");
        let other = dir.join("notes.command");
        for p in [&old, &fresh, &other] {
            fs::write(p, "x").unwrap();
        }
        let long_ago = SystemTime::now() - Duration::from_secs(3 * 60 * 60);
        for p in [&old, &other] {
            fs::File::options().write(true).open(p).unwrap().set_modified(long_ago).unwrap();
        }
        sweep_stale(dir);
        assert!(!old.exists());
        assert!(fresh.exists());
        assert!(other.exists());
        sweep_stale(&dir.join("missing")); // never errors
    }

    #[test]
    fn random_tokens_differ() {
        let a = random_token();
        let b = random_token();
        assert_eq!(a.len(), 12);
        assert!(a.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
