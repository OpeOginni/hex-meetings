//! Reads meetings through Hex's public, read-only command line
//! (`hex meetings list|show --json`, JSON version 1). Hex documents these
//! reads as never starting the app, opening the microphone, or writing
//! anything, so calling them is always safe. Nothing here reads Hex's
//! internal files directly: the CLI is the stable contract.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

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
