//! Session title = the first real user prompt in a transcript, cleaned.
//!
//! Contract (docs/ARCHITECTURE.md):
//! - Only `"type":"user"` lines count. `message.content` is a string, or an
//!   array whose first `{"type":"text"}` block supplies the text; a line
//!   carrying any `tool_result` block is not a prompt.
//! - Text whose trimmed form starts with `<` is skipped (`<command-name>`,
//!   `<local-command-stdout>`, `<system-reminder>`, …).
//! - Leading `[Pasted text #N +M lines]` markers are stripped (no regex),
//!   whitespace is collapsed, and the result is cut to 80 chars on a char
//!   boundary with a trailing `…`. Nothing longer than the title is kept.

use serde_json::Value;

/// Maximum title length in chars, including the ellipsis.
pub const MAX_TITLE_CHARS: usize = 80;

const PASTE_PREFIX: &str = "[Pasted text #";
/// Longest marker we will believe: `[Pasted text #123 +12345 lines]` is 31.
const PASTE_MAX_LEN: usize = 48;

/// Title candidate from one parsed transcript line, if it is a user prompt.
pub fn title_from_line(line: &Value) -> Option<String> {
    if line.get("type").and_then(Value::as_str) != Some("user") {
        return None;
    }
    let content = line.get("message")?.get("content")?;
    let raw = match content {
        Value::String(s) => s.as_str(),
        Value::Array(blocks) => {
            let kind = |b: &Value| b.get("type").and_then(Value::as_str).map(str::to_string);
            if blocks.iter().any(|b| kind(b).as_deref() == Some("tool_result")) {
                return None;
            }
            blocks
                .iter()
                .find(|b| kind(b).as_deref() == Some("text"))
                .and_then(|b| b.get("text"))
                .and_then(Value::as_str)?
        }
        _ => return None,
    };
    clean_title(raw)
}

/// Apply the cleaning rules to raw prompt text.
pub fn clean_title(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return None;
    }
    let collapsed = collapse_whitespace(strip_paste_markers(trimmed));
    if collapsed.is_empty() {
        return None;
    }
    Some(truncate_chars(&collapsed, MAX_TITLE_CHARS))
}

/// Remove every leading `[Pasted text #N +M lines]` marker.
pub fn strip_paste_markers(mut text: &str) -> &str {
    loop {
        text = text.trim_start();
        let Some(rest) = text.strip_prefix(PASTE_PREFIX) else { return text };
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return text;
        }
        let Some(close) = rest.find(']') else { return text };
        if close > PASTE_MAX_LEN {
            return text;
        }
        text = &rest[close + 1..];
    }
}

pub fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cut to `max` chars (not bytes); a cut string ends with `…` and is exactly `max` chars long.
pub fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let keep = max.saturating_sub(1);
    let mut out: String = text.chars().take(keep).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user_string(text: &str) -> Value {
        json!({"type":"user","message":{"role":"user","content":text},"cwd":"/Users/adz"})
    }

    #[test]
    fn string_content() {
        assert_eq!(
            title_from_line(&user_string("Please fix the build")).as_deref(),
            Some("Please fix the build")
        );
    }

    #[test]
    fn array_content_uses_first_text_block() {
        let line = json!({"type":"user","message":{"role":"user","content":[
            {"type":"image","source":{}},
            {"type":"text","text":"  what is   in this  image? "},
            {"type":"text","text":"second"}
        ]}});
        assert_eq!(title_from_line(&line).as_deref(), Some("what is in this image?"));
    }

    #[test]
    fn tool_result_lines_are_not_prompts() {
        let line = json!({"type":"user","message":{"role":"user","content":[
            {"type":"tool_result","tool_use_id":"t1","content":"ok"},
            {"type":"text","text":"not a prompt"}
        ]}});
        assert_eq!(title_from_line(&line), None);
    }

    #[test]
    fn non_user_and_angle_bracket_lines_are_skipped() {
        let assistant = json!({"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]}});
        assert_eq!(title_from_line(&assistant), None);
        let system = json!({"type":"system","subtype":"local_command","content":"<command-name>/resume</command-name>"});
        assert_eq!(title_from_line(&system), None);
        assert_eq!(title_from_line(&user_string("<command-name>/clear</command-name>")), None);
        assert_eq!(title_from_line(&user_string("  <local-command-stdout>x</local-command-stdout>")), None);
        assert_eq!(title_from_line(&user_string("<system-reminder>…</system-reminder>")), None);
        assert_eq!(title_from_line(&user_string("   ")), None);
        let no_message = json!({"type":"user"});
        assert_eq!(title_from_line(&no_message), None);
        let odd_content = json!({"type":"user","message":{"content":42}});
        assert_eq!(title_from_line(&odd_content), None);
    }

    #[test]
    fn pasted_text_marker_is_stripped() {
        assert_eq!(
            clean_title("[Pasted text #1 +120 lines] please summarise").as_deref(),
            Some("please summarise")
        );
        assert_eq!(
            clean_title("[Pasted text #2 +3 lines][Pasted text #3 +1 lines]\n do X").as_deref(),
            Some("do X")
        );
        assert_eq!(clean_title("[Pasted text #1 +120 lines]"), None, "pure paste yields no title");
        assert_eq!(clean_title("[Pasted text #] keep").as_deref(), Some("[Pasted text #] keep"));
        assert_eq!(clean_title("[Pasted things] keep").as_deref(), Some("[Pasted things] keep"));
        assert_eq!(strip_paste_markers("[Pasted text #1 +5 lines] tail"), "tail");
        let unterminated = "[Pasted text #1 +5 lines but no bracket ".to_string() + &"x".repeat(60) + "]";
        assert_eq!(strip_paste_markers(&unterminated), unterminated.as_str());
    }

    #[test]
    fn whitespace_is_collapsed() {
        assert_eq!(clean_title("fix\n\n  the\tbuild \r\n now").as_deref(), Some("fix the build now"));
    }

    #[test]
    fn truncates_on_char_boundary_to_80_chars() {
        let long = "é".repeat(100);
        let t = clean_title(&long).unwrap();
        assert_eq!(t.chars().count(), MAX_TITLE_CHARS);
        assert!(t.ends_with('…'));
        assert!(t.starts_with(&"é".repeat(79)));

        let cjk = "日本語のテキストをここに繰り返します".repeat(10);
        let t = clean_title(&cjk).unwrap();
        assert_eq!(t.chars().count(), 80);
        assert!(t.ends_with('…'));

        let exact = "a".repeat(80);
        assert_eq!(clean_title(&exact).as_deref(), Some(exact.as_str()));
        let over = "a".repeat(81);
        let t = clean_title(&over).unwrap();
        assert_eq!(t.chars().count(), 80);
        assert_eq!(&t[..79], &"a".repeat(79));
    }
}
