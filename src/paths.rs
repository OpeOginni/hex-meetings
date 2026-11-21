//! Where the app keeps its own files, and finding command-line tools.
//!
//! An app opened from Finder doesn't get the shell's PATH, so agents
//! installed through nvm, Homebrew or `~/.local/bin` (and the `node` some
//! of them run on) would not be found. [`adopt_login_path`] asks the login
//! shell for its PATH at launch, and [`search_path`] adds the usual folders.

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

/// The login shell's PATH, as a terminal would have it. None if the shell
/// fails or takes over three seconds (a slow or broken shell profile).
pub fn login_path() -> Option<std::ffi::OsString> {
    use std::process::{Command, Stdio};
    const MARK: &str = "__HEX_MEETINGS_PATH__";
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/zsh".into());
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c", &format!("printf '{MARK}%s{MARK}' \"$PATH\"")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut out = String::new();
        let _ = std::io::Read::read_to_string(&mut stdout, &mut out);
        let _ = tx.send(out);
    });
    let out = rx.recv_timeout(std::time::Duration::from_secs(3));
    let _ = child.kill();
    let _ = child.wait();
    let path = out.ok()?.split(MARK).nth(1)?.to_string();
    (!path.is_empty()).then(|| path.into())
}

static LOGIN_PATH: std::sync::OnceLock<Option<std::ffi::OsString>> = std::sync::OnceLock::new();

/// Opened from Finder (no terminal), reads the login shell's PATH once, so
/// [`search_path`] puts it first. Call at launch, before the window opens.
pub fn adopt_login_path() {
    LOGIN_PATH.get_or_init(|| if std::env::var_os("TERM").is_some() { None } else { login_path() });
}

/// The PATH children get: the login shell's (when opened from Finder), the
/// inherited one, then the usual install folders.
pub fn search_path() -> std::ffi::OsString {
    let home = home();
    let mut dirs: Vec<PathBuf> = LOGIN_PATH.get().cloned().flatten().map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    for dir in std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default() {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
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

#[cfg(test)]
mod tests {
    /// Needs a real login shell; run with `--ignored`.
    #[test]
    #[ignore]
    fn the_login_shell_gives_its_path() {
        let path = super::login_path().expect("the login shell answered");
        assert!(std::env::split_paths(&path).any(|d| d == std::path::Path::new("/usr/bin")), "{path:?}");
    }
}
