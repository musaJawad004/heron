//! Helpers for the JSON-lines files Claude Code writes: bounded reads,
//! lenient `serde_json::Value` field access, and the transcript-head parser.
//!
//! Contract:
//! - `read_head_lines` reads at most `max_bytes` from the start of a file,
//!   splits on `\n`, and drops a trailing partial line when the read was
//!   truncated. `read_tail` is the mirror image for append-only logs.
//! - `parse_head` extracts `cwd`, `gitBranch`, `version` and the title from a
//!   transcript head. It never keeps message bodies: only the cleaned title
//!   (see `title`) survives.
//! - Field access helpers accept numbers as integers, floats or numeric
//!   strings and treat empty strings as absent; nothing here panics.

use crate::claude::title::title_from_line;
use serde_json::Value;
use std::fs::{File, Metadata};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// How much of a transcript is ever read.
pub const HEAD_BYTES: u64 = 64 * 1024;

/// What the head of a transcript tells us about the session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadInfo {
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub version: Option<String>,
}

impl HeadInfo {
    fn is_complete(&self) -> bool {
        self.title.is_some() && self.cwd.is_some() && self.git_branch.is_some() && self.version.is_some()
    }
}

/// Parse the first `HEAD_BYTES` of a transcript.
pub fn read_head(path: &Path) -> io::Result<HeadInfo> {
    Ok(parse_head(&read_head_lines(path, HEAD_BYTES)?))
}

/// Complete lines from the first `max_bytes` of `path`.
pub fn read_head_lines(path: &Path, max_bytes: u64) -> io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let mut buf = Vec::with_capacity(max_bytes.min(HEAD_BYTES) as usize);
    file.by_ref().take(max_bytes).read_to_end(&mut buf)?;
    let truncated = buf.len() as u64 >= max_bytes;
    Ok(split_lines(&buf, truncated))
}

/// Split on `\n`, trimming `\r` and skipping blank lines. When `truncated`,
/// a final line without a newline is incomplete and dropped.
pub fn split_lines(buf: &[u8], truncated: bool) -> Vec<String> {
    let text = String::from_utf8_lossy(buf);
    let mut pieces: Vec<&str> = text.split('\n').collect();
    if let Some(last) = pieces.last() {
        if last.is_empty() || truncated {
            pieces.pop();
        }
    }
    pieces
        .into_iter()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// The last `max_bytes` of `path` as text; a leading partial line is dropped
/// when the file was longer than that.
pub fn read_tail(path: &Path, max_bytes: u64) -> io::Result<String> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut buf = Vec::with_capacity(len.min(max_bytes) as usize);
    let skipped = len > max_bytes;
    if skipped {
        file.seek(SeekFrom::Start(len - max_bytes))?;
    }
    file.read_to_end(&mut buf)?;
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if skipped {
        match text.find('\n') {
            Some(nl) => text.drain(..=nl),
            None => text.drain(..),
        };
    }
    Ok(text)
}

/// Extract session facts from transcript lines (already split).
pub fn parse_head(lines: &[String]) -> HeadInfo {
    let mut info = HeadInfo::default();
    for line in lines {
        let Ok(value) = serde_json::from_str::<Value>(line) else { continue };
        if info.cwd.is_none() {
            info.cwd = get_str(&value, "cwd").map(str::to_string);
        }
        if info.git_branch.is_none() {
            info.git_branch = get_str(&value, "gitBranch").map(str::to_string);
        }
        if info.version.is_none() {
            info.version = get_str(&value, "version").map(str::to_string);
        }
        if info.title.is_none() {
            info.title = title_from_line(&value);
        }
        if info.is_complete() {
            break;
        }
    }
    info
}

/// Non-empty string field.
pub fn get_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Unsigned integer field; accepts integer, non-negative float, or a numeric string.
pub fn get_u64(value: &Value, key: &str) -> Option<u64> {
    match value.get(key)? {
        Value::Number(n) => {
            n.as_u64().or_else(|| n.as_f64().filter(|f| f.is_finite() && *f >= 0.0).map(|f| f as u64))
        }
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// File modification time in Unix milliseconds.
pub fn mtime_ms(meta: &Metadata) -> Option<u64> {
    meta.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        File::create(&p).unwrap().write_all(bytes).unwrap();
        p
    }

    #[test]
    fn head_keeps_final_line_without_newline_when_not_truncated() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write(tmp.path(), "a.jsonl", b"{\"a\":1}\r\n\n{\"b\":2}");
        assert_eq!(read_head_lines(&p, HEAD_BYTES).unwrap(), vec!["{\"a\":1}", "{\"b\":2}"]);
    }

    #[test]
    fn head_drops_trailing_partial_line_when_truncated() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write(tmp.path(), "a.jsonl", b"{\"a\":1}\n{\"b\":2}\n{\"c\":333333}\n");
        // 20 bytes covers the first two lines and part of the third.
        assert_eq!(read_head_lines(&p, 20).unwrap(), vec!["{\"a\":1}", "{\"b\":2}"]);
        // Exactly at a newline boundary: still complete lines only.
        assert_eq!(read_head_lines(&p, 16).unwrap(), vec!["{\"a\":1}", "{\"b\":2}"]);
    }

    #[test]
    fn head_never_reads_more_than_the_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let mut big = Vec::new();
        for i in 0..5000 {
            big.extend_from_slice(format!("{{\"i\":{i},\"pad\":\"{}\"}}\n", "x".repeat(50)).as_bytes());
        }
        let p = write(tmp.path(), "big.jsonl", &big);
        let lines = read_head_lines(&p, HEAD_BYTES).unwrap();
        let total: usize = lines.iter().map(|l| l.len() + 1).sum();
        assert!(total <= HEAD_BYTES as usize);
        assert!(lines.len() > 900 && lines.len() < 1200, "got {}", lines.len());
    }

    #[test]
    fn tail_drops_leading_partial_line_only_when_cut() {
        let tmp = tempfile::tempdir().unwrap();
        let p = write(tmp.path(), "h.jsonl", b"{\"n\":1}\n{\"n\":2}\n{\"n\":3}\n");
        assert_eq!(read_tail(&p, 1 << 20).unwrap(), "{\"n\":1}\n{\"n\":2}\n{\"n\":3}\n");
        assert_eq!(read_tail(&p, 12).unwrap(), "{\"n\":3}\n");
        assert_eq!(read_tail(&p, 3).unwrap(), "");
    }

    #[test]
    fn parse_head_extracts_fields_and_skips_garbage() {
        let lines = vec![
            r#"{"type":"mode","mode":"normal","sessionId":"s"}"#.to_string(),
            "not json at all".to_string(),
            r#"{"parentUuid":null,"isSidechain":false,"promptId":"p","type":"user","message":{"role":"user","content":"Please fix the build"},"timestamp":"2026-09-04T13:37:41.586Z","uuid":"u","cwd":"/Users/you","sessionId":"s","version":"2.1.260","gitBranch":"main"}"#.to_string(),
        ];
        let info = parse_head(&lines);
        assert_eq!(info.cwd.as_deref(), Some("/Users/you"));
        assert_eq!(info.git_branch.as_deref(), Some("main"));
        assert_eq!(info.version.as_deref(), Some("2.1.260"));
        assert_eq!(info.title.as_deref(), Some("Please fix the build"));
        assert_eq!(parse_head(&[]), HeadInfo::default());
    }

    #[test]
    fn numeric_fields_are_lenient() {
        let v: Value = serde_json::json!({"a": 5, "b": 7.0, "c": "9", "d": -1, "e": "x", "f": null, "s": "", "t": " ok "});
        assert_eq!(get_u64(&v, "a"), Some(5));
        assert_eq!(get_u64(&v, "b"), Some(7));
        assert_eq!(get_u64(&v, "c"), Some(9));
        assert_eq!(get_u64(&v, "d"), None);
        assert_eq!(get_u64(&v, "e"), None);
        assert_eq!(get_u64(&v, "f"), None);
        assert_eq!(get_u64(&v, "missing"), None);
        assert_eq!(get_str(&v, "s"), None);
        assert_eq!(get_str(&v, "t"), Some("ok"));
        assert_eq!(get_str(&v, "a"), None);
    }
}
