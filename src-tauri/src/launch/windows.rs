//! Windows. Terminals: Windows Terminal (`wt.exe`), PowerShell (`pwsh.exe`
//! or `powershell.exe`) and `cmd.exe` (always present). The launch script is
//! a `.cmd`; console programs are started directly with `CREATE_NEW_CONSOLE`
//! and untouched stdio so the new console is theirs. Helper processes
//! (`powershell` for focus, `taskkill`) run with `CREATE_NO_WINDOW`.
//! Focus activates a window owned by the session pid, one of its ancestors,
//! a console host spawned by one of them, or (Windows Terminal hosts every
//! tab in one process) the newest Windows Terminal window.

use super::{find_on_path, powershell_quote, run_with_timeout, spawn_detached, LaunchError};
use crate::model::{Session, TerminalApp};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `wt.exe` is an app-execution alias (a reparse point), so `metadata` may
/// not resolve it; existence in any form is enough.
fn exists_any(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

fn wt() -> Option<PathBuf> {
    if let Some(path_var) = std::env::var_os("PATH") {
        let hit = std::env::split_paths(&path_var)
            .filter(|d| !d.as_os_str().is_empty())
            .map(|d| d.join("wt.exe"))
            .find(|p| exists_any(p));
        if hit.is_some() {
            return hit;
        }
    }
    let local = std::env::var_os("LOCALAPPDATA")?;
    let p = PathBuf::from(local).join("Microsoft").join("WindowsApps").join("wt.exe");
    exists_any(&p).then_some(p)
}

fn powershell() -> Option<PathBuf> {
    find_on_path("pwsh.exe").or_else(|| find_on_path("powershell.exe")).or_else(|| {
        let root = std::env::var_os("SystemRoot")?;
        let p = PathBuf::from(root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        p.is_file().then_some(p)
    })
}

fn comspec() -> PathBuf {
    std::env::var_os("COMSPEC").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("cmd.exe"))
}

pub fn installed_terminals() -> Vec<TerminalApp> {
    let mut list = Vec::new();
    if wt().is_some() {
        list.push(TerminalApp::WindowsTerminal);
    }
    if powershell().is_some() {
        list.push(TerminalApp::Powershell);
    }
    list.push(TerminalApp::Cmd);
    list
}

fn utf8<'a>(path: &'a Path, what: &str) -> Result<&'a str, LaunchError> {
    let s = path.to_str().ok_or_else(|| LaunchError::Other(format!("the {what} is not valid UTF-8")))?;
    if s.contains('"') {
        return Err(LaunchError::Other(format!("the {what} contains a double quote")));
    }
    Ok(s)
}

pub fn open_terminal(terminal: TerminalApp, cwd: &Path, script: &Path) -> Result<(), LaunchError> {
    let script_s = utf8(script, "launch script path")?;
    let cwd_s = utf8(cwd, "folder")?;
    let mut cmd = match terminal {
        TerminalApp::WindowsTerminal => {
            let wt = wt().ok_or(LaunchError::TerminalMissing(terminal))?;
            let mut cmd = Command::new(wt);
            // `;` separates wt subcommands; `\;` is the documented escape.
            cmd.arg("-d").arg(cwd_s.replace(';', "\\;")).args(["cmd", "/c"]).arg(script);
            cmd
        }
        TerminalApp::Powershell => {
            let ps = powershell().ok_or(LaunchError::TerminalMissing(terminal))?;
            let mut cmd = Command::new(ps);
            cmd.args(["-NoLogo", "-NoExit", "-Command"]).arg(format!("& {}", powershell_quote(script_s)));
            cmd.creation_flags(CREATE_NEW_CONSOLE);
            cmd
        }
        TerminalApp::Cmd => {
            let mut cmd = Command::new(comspec());
            // `/S` strips the outer quotes and leaves the inner ones intact.
            cmd.raw_arg(format!("/S /C \"\"{script_s}\"\""));
            cmd.creation_flags(CREATE_NEW_CONSOLE);
            cmd
        }
        _ => return Err(LaunchError::TerminalMissing(terminal)),
    };
    cmd.current_dir(cwd);
    spawn_detached(&mut cmd, Duration::from_millis(500), false)
}

/// PowerShell that activates the first window found up the process tree.
/// Only integers are interpolated.
fn focus_script(pid: u32) -> String {
    let me = std::process::id();
    format!(
        r#"$ErrorActionPreference = 'SilentlyContinue'
Add-Type -AssemblyName Microsoft.VisualBasic
$ids = New-Object System.Collections.Generic.List[int]
$id = {pid}
for ($i = 0; $i -lt 8 -and $id -gt 4 -and $id -ne {me}; $i++) {{
  $ids.Add($id)
  $p = Get-CimInstance Win32_Process -Filter "ProcessId = $id"
  if (-not $p) {{ break }}
  $id = [int]$p.ParentProcessId
}}
if ($ids.Count -eq 0) {{ exit 1 }}
foreach ($i in $ids) {{
  $proc = Get-Process -Id $i
  if ($proc -and $proc.MainWindowHandle -ne 0) {{
    try {{ [Microsoft.VisualBasic.Interaction]::AppActivate([int]$i); exit 0 }} catch {{}}
  }}
}}
foreach ($i in $ids) {{
  $hosts = Get-CimInstance Win32_Process -Filter "ParentProcessId = $i AND (Name = 'conhost.exe' OR Name = 'OpenConsole.exe')"
  foreach ($h in $hosts) {{
    try {{ [Microsoft.VisualBasic.Interaction]::AppActivate([int]$h.ProcessId); exit 0 }} catch {{}}
  }}
}}
$wt = Get-Process WindowsTerminal | Where-Object {{ $_.MainWindowHandle -ne 0 }} | Sort-Object StartTime -Descending | Select-Object -First 1
if ($wt) {{ try {{ [Microsoft.VisualBasic.Interaction]::AppActivate([int]$wt.Id); exit 0 }} catch {{}} }}
exit 1
"#
    )
}

pub fn focus(session: &Session, _terminal: TerminalApp) -> bool {
    let Some(pid) = session.pid.filter(|p| *p > 4) else { return false };
    let Some(ps) = powershell() else {
        log::debug!("focus: no PowerShell available");
        return false;
    };
    let mut cmd = Command::new(ps);
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &focus_script(pid)]);
    cmd.creation_flags(CREATE_NO_WINDOW);
    match run_with_timeout(&mut cmd, Duration::from_secs(6)) {
        Ok(Some(out)) if out.success() => true,
        Ok(Some(out)) => {
            log::debug!("focus pid {pid}: no window activated {}", out.stderr_text());
            false
        }
        Ok(None) => {
            log::warn!("focus: PowerShell did not answer within 6 s");
            false
        }
        Err(e) => {
            log::warn!("focus: {e}");
            false
        }
    }
}

fn taskkill(args: &[&str]) -> Result<bool, LaunchError> {
    let mut cmd = Command::new("taskkill");
    cmd.args(args).creation_flags(CREATE_NO_WINDOW);
    match run_with_timeout(&mut cmd, Duration::from_secs(5))? {
        Some(out) => Ok(out.success()),
        None => Err(LaunchError::Other("taskkill did not finish".into())),
    }
}

/// Polite first (`WM_CLOSE`); console programs usually refuse that, so fall
/// back to `/F`.
pub fn terminate(pid: u32) -> Result<(), LaunchError> {
    let pid_s = pid.to_string();
    if taskkill(&["/PID", &pid_s])? {
        return Ok(());
    }
    log::info!("process {pid} did not accept a polite close; forcing");
    if taskkill(&["/F", "/PID", &pid_s])? {
        Ok(())
    } else {
        Err(LaunchError::Other(format!("could not end process {pid}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_is_always_offered_last() {
        let list = installed_terminals();
        assert_eq!(list.last(), Some(&TerminalApp::Cmd));
    }

    #[test]
    fn focus_script_only_embeds_integers() {
        let s = focus_script(1234);
        assert!(s.contains("$id = 1234\n"));
        assert!(s.contains(&format!("-ne {}", std::process::id())));
    }

    #[test]
    fn open_rejects_unix_terminals_and_quotes() {
        let err =
            open_terminal(TerminalApp::Terminal, Path::new("C:\\"), Path::new("C:\\x.cmd")).unwrap_err();
        assert!(matches!(err, LaunchError::TerminalMissing(TerminalApp::Terminal)));
        let err = open_terminal(TerminalApp::Cmd, Path::new("C:\\"), Path::new("C:\\a\"b.cmd")).unwrap_err();
        assert!(matches!(err, LaunchError::Other(_)));
    }
}
