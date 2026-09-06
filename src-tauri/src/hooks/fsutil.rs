//! Small filesystem helpers shared by the hooks module: private directories
//! and files (0700 / 0600 on Unix), atomic replace, a UTC timestamp for
//! backup names, and the quoting used when a path is baked into a script.
//! Nothing here knows about Claude Code or Tauri.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// `create_dir_all` followed by `chmod 700` on Unix.
pub(super) fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    set_mode(dir, 0o700);
    Ok(())
}

/// Apply a Unix permission mode; a no-op elsewhere. A failure is ignored on
/// purpose: the path already exists and a filesystem that refuses `chmod`
/// (some network mounts) should not turn into a hard error.
pub(super) fn set_mode(path: &Path, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
}

/// Permission bits of `path` (Unix only; `None` elsewhere or when missing).
pub(super) fn mode_of(path: &Path) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).ok().map(|m| m.permissions().mode() & 0o777)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

/// Write `bytes` to a sibling temp file with `mode`, then rename it over
/// `path`, so a concurrent reader sees either the old or the new content.
pub(super) fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut tmp_name = name.to_os_string();
    tmp_name.push(format!(".heron-tmp-{}", std::process::id()));
    let tmp = dir.join(tmp_name);
    let result = write_private(&tmp, bytes, mode).and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_private(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    use std::io::Write;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(mode);
    }
    let mut file = opts.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    // `OpenOptions::mode` is subject to the umask; make the mode exact.
    set_mode(path, mode);
    Ok(())
}

/// Seconds since the Unix epoch (0 if the clock is before 1970).
pub(super) fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `yyyyMMdd-HHmmss` in UTC for a Unix timestamp in seconds.
pub(super) fn utc_stamp(secs: u64) -> String {
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    let rem = secs % 86_400;
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rem / 3_600, (rem % 3_600) / 60, rem % 60)
}

/// Days since 1970-01-01 to a proleptic Gregorian (year, month, day).
/// Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// POSIX single-quoted literal: only `'` is special and becomes `'\''`.
pub(super) fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// PowerShell single-quoted literal: only `'` is special and is doubled.
pub(super) fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_stamp_matches_known_dates() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(951_782_400), "20000229-000000");
        assert_eq!(utc_stamp(1_704_067_200), "20240101-000000");
        assert_eq!(utc_stamp(1_788_684_708), "20260906-085148");
        assert_eq!(utc_stamp(4_102_444_799), "20991231-235959");
    }

    #[test]
    fn quoting() {
        assert_eq!(sh_quote("/a b/c"), "'/a b/c'");
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
        assert_eq!(ps_quote("C:\\Users\\o'brien"), "'C:\\Users\\o''brien'");
    }

    #[test]
    fn atomic_write_replaces_and_sets_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        atomic_write(&path, b"one", 0o600).unwrap();
        atomic_write(&path, b"two", 0o600).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        assert!(fs::read_dir(dir.path()).unwrap().count() == 1, "no temp file left behind");
        #[cfg(unix)]
        assert_eq!(mode_of(&path), Some(0o600));
    }
}
