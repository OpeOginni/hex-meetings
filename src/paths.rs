//! Where the app keeps its own files, and finding command-line tools.
//!
//! An app opened from Finder doesn't get the shell's PATH, so agents
//! installed in the usual per-user folders (`~/.opencode/bin`,
//! `~/.local/bin`, Homebrew) would not be found. [`search_path`] adds them.

use std::path::PathBuf;

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// `~/Library/Application Support/hex-meetings` on macOS, the XDG data
/// folder elsewhere. `HEX_MEETINGS_DIR` overrides it (tests, demos).
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("HEX_MEETINGS_DIR") {
        return PathBuf::from(dir);
    }
    if cfg!(target_os = "macos") {
        return home().join("Library/Application Support/hex-meetings");
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"))
        .join("hex-meetings")
}

pub fn reports_dir() -> PathBuf {
    data_dir().join("reports")
}

pub fn activity_file() -> PathBuf {
    data_dir().join("activity.jsonl")
}

pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

/// Where results are saved as Markdown.
pub fn summaries_dir() -> PathBuf {
    home().join("Documents/Meeting summaries")
}

/// A path with the home folder shown as `~`.
pub fn tilde(path: &std::path::Path) -> String {
    match path.strip_prefix(home()) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The PATH children get: the inherited one, then the usual install folders.
pub fn search_path() -> std::ffi::OsString {
    let home = home();
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for extra in [
        home.join(".opencode/bin"),
        home.join(".local/bin"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        home.join(".npm-global/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ] {
        if !dirs.contains(&extra) {
            dirs.push(extra);
        }
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

/// The first executable named `name` on [`search_path`].
pub fn which(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&search_path())
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}
