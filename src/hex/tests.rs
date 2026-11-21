use super::*;

// The shapes Hex 3.4 prints for `meetings list --json` and `meetings show --json`.
const LIST: &str = r#"{
  "meetings": [
    { "application": null, "duration_ms": 10000, "id": "1790001000000-1003", "in_progress": false,
      "status": "interrupted", "timestamp_ms": 1790001000000, "title": "Standup",
      "transcript_source": "rolling", "warnings": ["skipped 1 unreadable lines in transcript.rolling.ndjson"] },
    { "application": "Zoom", "duration_ms": 40000, "id": "1790000000000-1001", "in_progress": false,
      "status": "complete", "timestamp_ms": 1790000000000, "title": "Weekly sync",
      "transcript_source": "final", "warnings": [], "a_field_from_a_newer_hex": true }
  ],
  "skipped": [{ "id": "1790001500000-1004", "reason": "could not decode manifest.json" }],
  "truncated": false,
  "version": 1
}"#;

const SHOW: &str = r#"{
  "entries": [
    { "end_ms": 4000, "source": "system", "start_ms": 0, "text": "Thanks for making time." },
    { "end_ms": 9000, "source": "microphone", "start_ms": 4500, "text": " Of course. " }
  ],
  "meeting": { "application": "Zoom", "duration_ms": 40000, "id": "1790000000000-1001",
    "in_progress": false, "status": "complete", "timestamp_ms": 1790000000000,
    "title": "Weekly sync", "transcript_source": "final", "warnings": [] },
  "notes": { "generated_at_ms": 1790000100000, "summary": "A short sync.", "title": "Sync" },
  "version": 1
}"#;

#[test]
fn reads_hex_list_output_and_ignores_unknown_fields() {
    let meetings = parse_list(LIST).unwrap();
    assert_eq!(meetings.len(), 2);
    assert_eq!(meetings[0].application, None);
    assert_eq!(meetings[0].status, "interrupted");
    assert_eq!(meetings[1].application.as_deref(), Some("Zoom"));
}

#[test]
fn reads_hex_show_output_with_speakers() {
    let detail = parse_show(SHOW).unwrap();
    assert_eq!(detail.notes.as_ref().unwrap().summary, "A short sync.");
    assert_eq!(detail.entries[0].speaker(), "Them");
    assert_eq!(detail.entries[1].speaker(), "You");
    assert_eq!(transcript_text(&detail.entries), "[00:00] Them: Thanks for making time.\n[00:04] You: Of course.\n");
}

#[test]
fn refuses_a_json_version_it_does_not_know() {
    let err = parse_list(r#"{"version": 2, "meetings": []}"#).unwrap_err();
    assert!(err.contains("version 2"), "{err}");
}

/// Checks the contract against the Hex installed on this machine, with an
/// empty data folder so no real meeting is read. Skipped without Hex.
#[test]
fn the_installed_hex_answers_in_the_format_this_app_reads() {
    let Some(hex) = find_binary() else { return };
    let dir = std::env::temp_dir().join(format!("hex-meetings-empty-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("meetings")).unwrap();
    let out = std::process::Command::new(&hex)
        .args(["meetings", "list", "--since", "365d", "--limit", "200", "--json"])
        .env("HEX_APPLICATION_SUPPORT_DIR", &dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(parse_list(&String::from_utf8(out.stdout).unwrap()).unwrap(), Vec::new());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn formats_times_and_lengths() {
    assert_eq!(clock(65_000), "01:05");
    assert_eq!(clock(3_725_000), "1:02:05");
    assert_eq!(duration(45_000), "45 s");
    assert_eq!(duration(12 * 60_000), "12 min");
    assert_eq!(duration(65 * 60_000), "1 h 05 min");
}
