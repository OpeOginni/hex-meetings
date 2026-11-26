//! Talks to Hex through its public command line, never its files. Meetings
//! are read with `hex meetings list|show --json` (JSON version 1), which Hex
//! documents as never starting the app, opening the microphone, or writing
//! anything. Only on the person's request: `hex settings` for how long Hex
//! keeps calls, and `hex meetings delete` to move one to the Trash.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::speech::SpeechModel;

/// Where to find the `hex` binary: `HEX_MEETINGS_HEX`, then `hex` on the
/// PATH (or the usual install folders), then the app bundle.
pub fn find_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HEX_MEETINGS_HEX") {
        return Some(PathBuf::from(path));
    }
    if let Some(path) = crate::paths::which("hex") {
        return Some(path);
    }
    let bundle = Path::new("/Applications/Hex.app/Contents/MacOS/hex");
    bundle.exists().then(|| bundle.to_path_buf())
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Meeting {
    pub id: String,
    pub title: String,
    pub timestamp_ms: i64,
    pub duration_ms: u64,
    pub application: Option<String>,
    pub status: String,
    #[serde(default)]
    pub in_progress: bool,
    #[serde(default)]
    pub transcript_source: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MeetingList {
    version: u32,
    meetings: Vec<Meeting>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Notes {
    pub title: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Entry {
    /// `microphone` (You) or `system` (Them).
    pub source: String,
    pub start_ms: u64,
    #[serde(default)]
    pub end_ms: u64,
    pub text: String,
}

impl Entry {
    pub fn speaker(&self) -> &'static str {
        if self.source == "microphone" { "You" } else { "Them" }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Detail {
    pub meeting: Meeting,
    pub notes: Option<Notes>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Show {
    version: u32,
    #[serde(flatten)]
    detail: Detail,
}

pub fn parse_list(json: &str) -> Result<Vec<Meeting>, String> {
    let list: MeetingList = serde_json::from_str(json).map_err(|e| format!("Hex answered in an unexpected format: {e}"))?;
    if list.version != 1 {
        return Err(format!("Hex answered with JSON version {}; this app reads version 1.", list.version));
    }
    Ok(list.meetings)
}

pub fn parse_show(json: &str) -> Result<Detail, String> {
    let show: Show = serde_json::from_str(json).map_err(|e| format!("Hex answered in an unexpected format: {e}"))?;
    if show.version != 1 {
        return Err(format!("Hex answered with JSON version {}; this app reads version 1.", show.version));
    }
    Ok(show.detail)
}

fn run(hex: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new(hex)
        .args(args)
        .output()
        .map_err(|e| format!("Could not run {}: {e}", hex.display()))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("hex {} failed: {}", args.join(" "), err.trim()));
    }
    String::from_utf8(out.stdout).map_err(|_| "Hex printed text that isn't UTF-8.".to_string())
}

/// The newest 200 meetings of the last year, newest first.
pub fn list(hex: &Path) -> Result<Vec<Meeting>, String> {
    parse_list(&run(hex, &["meetings", "list", "--since", "365d", "--limit", "200", "--json"])?)
}

pub fn show(hex: &Path, id: &str) -> Result<Detail, String> {
    parse_show(&run(hex, &["meetings", "show", id, "--json"])?)
}

/// Where people download Hex (Apple silicon, macOS 15 or later).
pub const DOWNLOAD_PAGE: &str = "https://anoma.ly/hex";

/// Settings by key from `hex settings get <key or prefix> --json`. Hex
/// answers settings only while it's running.
fn settings(hex: &Path, key: &str) -> Result<BTreeMap<String, serde_json::Value>, String> {
    parse_settings(&run(hex, &["settings", "get", key, "--json"])?)
}

pub fn parse_settings(json: &str) -> Result<BTreeMap<String, serde_json::Value>, String> {
    #[derive(Deserialize)]
    struct Setting {
        key: String,
        #[serde(default)]
        value: serde_json::Value,
    }
    #[derive(Deserialize)]
    struct Reply {
        settings: Vec<Setting>,
    }
    let reply: Reply = serde_json::from_str(json).map_err(|e| format!("Hex answered in an unexpected format: {e}"))?;
    Ok(reply.settings.into_iter().map(|s| (s.key, s.value)).collect())
}

/// Whether Labs › Meetings is on, so Hex records calls.
pub fn meetings_enabled(hex: &Path) -> Result<bool, String> {
    settings(hex, "meetings.enabled")?.get("meetings.enabled").and_then(|v| v.as_bool()).ok_or_else(|| "Hex didn't say whether Meetings is on.".into())
}

/// Hex's meeting settings and speech models, for the Setup page and the
/// language check. Read-only.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overview {
    /// Every `meetings.*` setting.
    pub settings: BTreeMap<String, serde_json::Value>,
    pub models: Vec<SpeechModel>,
}

impl Overview {
    fn text(&self, key: &str) -> Option<&str> {
        self.settings.get(key).and_then(|v| v.as_str())
    }

    /// `day`, `week`, `month`, `forever` or `none`; None while unset.
    pub fn retention(&self) -> Option<&str> {
        self.text("meetings.retention")
    }

    /// When Hex trashes a finished call's audio: `day`, `week`, `month`, or
    /// `none` (with the rest of the call); None while unset.
    pub fn audio_retention(&self) -> Option<&str> {
        self.text("meetings.audio_retention")
    }

    /// Whether Labs › More call apps is on; None when Hex didn't say.
    pub fn more_call_apps(&self) -> Option<bool> {
        self.settings.get("labs.more_call_apps").and_then(|v| v.as_bool())
    }

}

pub fn overview(hex: &Path) -> Result<Overview, String> {
    #[derive(Deserialize)]
    struct Models {
        models: Vec<SpeechModel>,
    }
    let models: Models = serde_json::from_str(&run(hex, &["model", "list", "--json"])?).map_err(|e| format!("Hex listed its models in an unexpected format: {e}"))?;
    let mut all = settings(hex, "meetings")?;
    // Whether Labs › More call apps is on; an older Hex may not have it.
    if let Ok(labs) = settings(hex, "labs.more_call_apps") {
        all.extend(labs);
    }
    Ok(Overview { settings: all, models: models.models })
}

/// Opens Hex's settings with the speech model picker.
pub fn open_models(hex: &Path) -> Result<(), String> {
    run(hex, &["start"])?;
    run(hex, &["window", "show", "models"]).map(drop)
}

/// Opens Hex if needed, turns on Labs › Meetings, and shows Hex's Meetings
/// pane, where Hex asks for the microphone and system audio.
pub fn turn_on_meetings(hex: &Path) -> Result<(), String> {
    run(hex, &["start"])?;
    run(hex, &["labs", "on", "meetings"])?;
    run(hex, &["window", "show", "meetings"]).map(drop)
}

/// Changes `meetings.retention` or `meetings.audio_retention`, as Hex's own
/// window would.
pub fn set_keep(hex: &Path, key: &str, value: &str) -> Result<(), String> {
    run(hex, &["settings", "set", key, value]).map(drop)
}

/// What Hex does with finished calls, for a given `meetings.retention`.
pub fn retention_sentence(value: Option<&str>) -> &'static str {
    match value {
        Some("day") => "Hex keeps calls for a day, then moves them to the Trash. Their Markdown files stay.",
        Some("week") => "Hex keeps calls for a week, then moves them to the Trash. Their Markdown files stay.",
        Some("month") => "Hex keeps calls for a month, then moves them to the Trash. Their Markdown files stay.",
        Some("forever") => "Hex keeps calls until you delete them.",
        // Unset or `none`: Hex keeps new calls 7 days.
        _ => "Hex keeps calls for 7 days, then moves them to the Trash. Their Markdown files stay.",
    }
}

/// What Hex does with a finished call's audio, for `meetings.audio_retention`.
pub fn audio_sentence(value: Option<&str>) -> &'static str {
    match value {
        Some("day") => "Hex moves a call's audio to the Trash after a day. Its transcript stays.",
        Some("week") => "Hex moves a call's audio to the Trash after a week. Its transcript stays.",
        Some("month") => "Hex moves a call's audio to the Trash after a month. Its transcript stays.",
        _ => "Hex keeps a call's audio as long as the call.",
    }
}

/// Opens Hex (if needed) on its Meetings pane, where the rest of Hex's
/// meeting settings are.
pub fn open_meetings(hex: &Path) -> Result<(), String> {
    run(hex, &["start"])?;
    run(hex, &["window", "show", "meetings"]).map(drop)
}

/// Moves a finished meeting's Markdown and recording to the Trash, as the
/// Meetings pane's Delete does. Hex asks no confirmation, so the caller asks
/// first. Hex must be running, so this opens it if needed.
pub fn delete(hex: &Path, id: &str) -> Result<(), String> {
    run(hex, &["start"])?;
    run(hex, &["meetings", "delete", id]).map(drop)
}

pub fn version(hex: &Path) -> Option<String> {
    let out = run(hex, &["--version"]).ok()?;
    Some(out.trim().trim_start_matches("hex ").to_string())
}

/// `mm:ss`, or `h:mm:ss` past an hour.
pub fn clock(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

/// `45 s`, `12 min`, `1 h 05 min`.
pub fn duration(ms: u64) -> String {
    let s = ms / 1000;
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} min", s / 60),
        _ => format!("{} h {:02} min", s / 3600, (s / 60) % 60),
    }
}

/// The transcript as `[mm:ss] You: text` lines, the form agents read best
/// and the one Hex's own MCP server uses.
pub fn transcript_text(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str(&format!("[{}] {}: {}\n", clock(e.start_ms), e.speaker(), e.text.trim()));
    }
    out
}

#[cfg(test)]
mod tests;
