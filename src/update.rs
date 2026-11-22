//! GitHub release checks and app bundle replacement. Network calls must run off the UI thread.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::Deserialize;

pub const RELEASES: &str = "https://api.github.com/repos/OpeOginni/hex-meetings/releases/latest";
// These asset names must match scripts/package-macos.sh.
pub const ZIP: &str = "Hex-Meetings-macOS-arm64.zip";
const SUMS: &str = "SHA256SUMS";

pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub page: String,
    pub zip: Option<String>,
    pub sums: Option<String>,
}

pub fn parse_release(json: &str) -> Result<Release, String> {
    #[derive(Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
    }
    #[derive(Deserialize)]
    struct Raw {
        tag_name: String,
        html_url: String,
        #[serde(default)]
        assets: Vec<Asset>,
    }
    let raw: Raw = serde_json::from_str(json).map_err(|e| format!("GitHub answered in an unexpected format: {e}"))?;
    let asset = |name: &str| raw.assets.iter().find(|a| a.name == name).map(|a| a.browser_download_url.clone());
    Ok(Release {
        version: raw.tag_name.trim_start_matches('v').to_string(),
        page: raw.html_url.clone(),
        zip: asset(ZIP),
        sums: asset(SUMS),
    })
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.split(['.', '-', '+']).map_while(|p| p.parse().ok()).collect() };
    parts(latest) > parts(current)
}

fn get(url: &str) -> ureq::Request {
    ureq::get(url).set("User-Agent", "hex-meetings").timeout(Duration::from_secs(60))
}

pub fn latest() -> Result<Release, String> {
    let reply = get(RELEASES).set("Accept", "application/vnd.github+json").call().map_err(|e| match e {
        ureq::Error::Status(404, _) => "There's no release on GitHub yet.".to_string(),
        ureq::Error::Status(code, _) => format!("GitHub answered {code}. Try again later."),
        ureq::Error::Transport(_) => "Couldn't reach GitHub. Check your connection.".to_string(),
    })?;
    parse_release(&reply.into_string().map_err(|e| e.to_string())?)
}

/// Returns `None` for a binary run directly from `target/`.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(Path::to_path_buf)
}

pub fn install(release: &Release) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bundle = bundle_of(&exe).ok_or("This copy runs from a build folder, so it can't update itself. Build it again instead.")?;
    install_into(release, &bundle).map(|()| bundle)
}

pub fn install_into(release: &Release, bundle: &Path) -> Result<(), String> {
    let (Some(zip_url), Some(sums_url)) = (&release.zip, &release.sums) else {
        return Err(format!("Version {} has no Mac download yet. Open its release page instead.", release.version));
    };
    let work = std::env::temp_dir().join(format!("hex-meetings-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        let zip = work.join(ZIP);
        let mut file = std::fs::File::create(&zip).map_err(|e| e.to_string())?;
        let reply = get(zip_url).timeout(Duration::from_secs(600)).call().map_err(|e| format!("The download failed: {e}"))?;
        std::io::copy(&mut reply.into_reader(), &mut file).map_err(|e| format!("The download failed: {e}"))?;
        let sums = get(sums_url).call().map_err(|e| format!("Couldn't read the release's checksums: {e}"))?.into_string().map_err(|e| e.to_string())?;
        swap(bundle, &zip, &sums)
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

fn sha256(file: &Path) -> Result<String, String> {
    let out = Command::new("/usr/bin/shasum").args(["-a", "256"]).arg(file).output().map_err(|e| e.to_string())?;
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().map(str::to_string).ok_or("Couldn't check the download.".into())
}

/// Verify the checksum before touching the installed bundle.
pub fn swap(bundle: &Path, zip: &Path, sums: &str) -> Result<(), String> {
    let name = zip.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let expected = sums
        .lines()
        .find_map(|l| l.split_once(char::is_whitespace).filter(|(_, file)| file.trim().trim_start_matches('*') == name).map(|(sum, _)| sum.to_lowercase()))
        .ok_or("The release's checksums don't list the download.")?;
    if sha256(zip)? != expected {
        return Err("The download doesn't match the release's checksum, so it wasn't installed.".into());
    }
    let parent = bundle.parent().ok_or("The app has no folder.")?;
    let unpacked = parent.join(".Hex Meetings update");
    let old = parent.join(".Hex Meetings previous");
    for dir in [&unpacked, &old] {
        let _ = std::fs::remove_dir_all(dir);
    }
    let not_writable = |e: std::io::Error| {
        format!("Couldn't write to {} ({e}). Run the install command from the README instead.", parent.display())
    };
    std::fs::create_dir_all(&unpacked).map_err(not_writable)?;
    let status = Command::new("/usr/bin/ditto").arg("-x").arg("-k").arg(zip).arg(&unpacked).status().map_err(|e| e.to_string())?;
    let new_app = unpacked.join("Hex Meetings.app");
    if !status.success() || !new_app.join("Contents/MacOS/Hex Meetings").is_file() {
        let _ = std::fs::remove_dir_all(&unpacked);
        return Err("The download doesn't contain Hex Meetings.app.".into());
    }
    std::fs::rename(bundle, &old).map_err(not_writable)?;
    if let Err(e) = std::fs::rename(&new_app, bundle) {
        let _ = std::fs::rename(&old, bundle);
        let _ = std::fs::remove_dir_all(&unpacked);
        return Err(not_writable(e));
    }
    let _ = std::fs::remove_dir_all(&unpacked);
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

/// Opens `bundle` again once this process has quit, then quits.
pub fn relaunch(bundle: &Path) -> ! {
    let _ = Command::new("/bin/sh")
        .args(["-c", "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; open \"$2\"", "relaunch"])
        .arg(std::process::id().to_string())
        .arg(bundle)
        .spawn();
    std::process::exit(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.1"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn a_release_names_its_version_page_and_downloads() {
        let json = r#"{"tag_name": "v0.2.0", "html_url": "https://github.com/OpeOginni/hex-meetings/releases/tag/v0.2.0",
            "assets": [{"name": "Hex-Meetings-macOS-arm64.dmg", "browser_download_url": "https://x/dmg"},
                       {"name": "Hex-Meetings-macOS-arm64.zip", "browser_download_url": "https://x/zip"},
                       {"name": "SHA256SUMS", "browser_download_url": "https://x/sums"}], "body": "Notes"}"#;
        let r = parse_release(json).unwrap();
        assert_eq!(r.version, "0.2.0");
        assert_eq!(r.zip.as_deref(), Some("https://x/zip"));
        assert_eq!(r.sums.as_deref(), Some("https://x/sums"));
        let bare = parse_release(r#"{"tag_name": "v0.3.0", "html_url": "https://x"}"#).unwrap();
        assert!(bare.zip.is_none());
    }

    #[test]
    fn only_an_app_bundle_can_update() {
        assert_eq!(
            bundle_of(Path::new("/Applications/Hex Meetings.app/Contents/MacOS/Hex Meetings")).as_deref(),
            Some(Path::new("/Applications/Hex Meetings.app"))
        );
        assert_eq!(bundle_of(Path::new("/Users/me/hex-meetings/target/debug/hex-meetings")), None);
    }

    fn release_zip(dir: &Path, marker: &str) -> PathBuf {
        let app = dir.join("src/Hex Meetings.app/Contents/MacOS");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("Hex Meetings"), marker).unwrap();
        let zip = dir.join(ZIP);
        let ok = Command::new("/usr/bin/ditto")
            .args(["-c", "-k", "--keepParent"])
            .arg(dir.join("src/Hex Meetings.app"))
            .arg(&zip)
            .status()
            .unwrap();
        assert!(ok.success());
        zip
    }

    #[test]
    fn swap_replaces_the_app_only_when_the_checksum_matches() {
        let dir = std::env::temp_dir().join(format!("hex-meetings-swap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let installed = dir.join("Applications/Hex Meetings.app");
        std::fs::create_dir_all(installed.join("Contents/MacOS")).unwrap();
        std::fs::write(installed.join("Contents/MacOS/Hex Meetings"), "old").unwrap();
        let zip = release_zip(&dir, "new");
        let read = || std::fs::read_to_string(installed.join("Contents/MacOS/Hex Meetings")).unwrap();

        let wrong = format!("{}  {ZIP}\n", "0".repeat(64));
        assert!(swap(&installed, &zip, &wrong).unwrap_err().contains("doesn't match"));
        assert_eq!(read(), "old", "a bad download leaves the app alone");

        let sums = format!("{}  {ZIP}\nabc  other.dmg\n", sha256(&zip).unwrap());
        swap(&installed, &zip, &sums).unwrap();
        assert_eq!(read(), "new");
        let left: Vec<_> = std::fs::read_dir(dir.join("Applications")).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, ["Hex Meetings.app"], "nothing is left beside the app");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore]
    fn real_update_installs_the_latest_release() {
        let release = latest().unwrap();
        println!("{release:?}");
        let dir = std::env::temp_dir().join(format!("hex-meetings-real-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bundle = dir.join("Hex Meetings.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::write(bundle.join("Contents/MacOS/Hex Meetings"), "old").unwrap();
        install_into(&release, &bundle).unwrap();
        let plist = std::fs::read_to_string(bundle.join("Contents/Info.plist")).unwrap();
        assert!(plist.contains(&format!("<string>{}</string>", release.version)), "the installed app is {}", release.version);
        let quarantine = Command::new("/usr/bin/xattr").args(["-p", "com.apple.quarantine"]).arg(&bundle).output().unwrap();
        assert!(!quarantine.status.success(), "no quarantine flag, so macOS doesn't block it");
        let signed = Command::new("/usr/bin/codesign").args(["--verify", "--strict"]).arg(&bundle).status().unwrap();
        assert!(signed.success(), "the signature survives the download");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
