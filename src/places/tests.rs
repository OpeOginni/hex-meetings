use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use super::*;

/// One recorded request: its first line, headers (lowercased names) and body.
struct Seen {
    line: String,
    headers: Vec<(String, String)>,
    body: Value,
}

impl Seen {
    fn header(&self, name: &str) -> &str {
        self.headers.iter().find(|(n, _)| n == name).map_or("", |(_, v)| v.as_str())
    }
}

/// A local HTTP server that answers each request with the next reply and
/// hands back what it received.
fn server(replies: Vec<&'static str>) -> (String, mpsc::Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let mut headers = Vec::new();
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).unwrap();
                let h = h.trim_end();
                if h.is_empty() {
                    break;
                }
                let (n, v) = h.split_once(':').unwrap();
                headers.push((n.to_lowercase(), v.trim().to_string()));
            }
            let len: usize = headers.iter().find(|(n, _)| n == "content-length").map_or(0, |(_, v)| v.parse().unwrap());
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            // "404 body" answers with that status; anything else is a 200.
            let (status, reply) = match reply.split_once(' ').filter(|(c, _)| c.len() == 3 && c.chars().all(|ch| ch.is_ascii_digit())) {
                Some((code, rest)) => (code.to_string(), rest),
                None => ("200".to_string(), reply),
            };
            write!(stream, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).unwrap();
            let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
            tx.send(Seen { line: line.trim().to_string(), headers, body }).unwrap();
        }
    });
    (base, rx)
}

#[test]
fn github_gets_one_issue_per_item_with_the_token() {
    let (base, seen) = server(vec![r#"{"html_url": "https://github.com/acme/app/issues/7"}"#, r#"{"html_url": "https://github.com/acme/app/issues/8"}"#]);
    let at = Endpoints { github: base, linear: String::new(), notion: String::new() };
    let conn = Connection { place: Place::GitHub, target: "acme/app".into() };
    let reply = "```json\n[{\"title\": \"Export times out\", \"body\": \"Above 50k rows.\"}, {\"title\": \"Dates in US format\"}]\n```";
    let links = post(&conn, "secret-token", "", reply, &at).unwrap();
    assert_eq!(links, ["https://github.com/acme/app/issues/7", "https://github.com/acme/app/issues/8"]);
    let first = seen.recv().unwrap();
    assert_eq!(first.line, "POST /repos/acme/app/issues HTTP/1.1");
    assert_eq!(first.header("authorization"), "Bearer secret-token");
    assert_eq!(first.body, json!({ "title": "Export times out", "body": "Above 50k rows." }));
    assert_eq!(seen.recv().unwrap().body["title"], "Dates in US format");
}

#[test]
fn linear_finds_the_team_then_creates_the_issues() {
    let (base, seen) = server(vec![
        r#"{"data": {"teams": {"nodes": [{"id": "team-1"}]}}}"#,
        r#"{"data": {"issueCreate": {"issue": {"url": "https://linear.app/acme/issue/ENG-1"}}}}"#,
    ]);
    let at = Endpoints { github: String::new(), linear: format!("{base}/graphql"), notion: String::new() };
    let conn = Connection { place: Place::Linear, target: "eng".into() };
    let links = post(&conn, "lin_api_x", "", r#"[{"title": "Export times out", "body": "Above 50k rows."}]"#, &at).unwrap();
    assert_eq!(links, ["https://linear.app/acme/issue/ENG-1"]);
    let team = seen.recv().unwrap();
    assert_eq!(team.header("authorization"), "lin_api_x");
    assert_eq!(team.body["variables"], json!({ "key": "ENG" }));
    let made = seen.recv().unwrap();
    assert_eq!(made.body["variables"]["input"], json!({ "teamId": "team-1", "title": "Export times out", "description": "Above 50k rows." }));
}

#[test]
fn slack_gets_the_message_as_text() {
    let (base, seen) = server(vec!["ok"]);
    let conn = Connection { place: Place::Slack, target: "#meeting-notes".into() };
    assert!(post(&conn, &format!("{base}/services/T/B/x"), "", "  *Northwind* call\n• Send the plan  ", &Endpoints::default()).unwrap().is_empty());
    let s = seen.recv().unwrap();
    assert_eq!(s.line, "POST /services/T/B/x HTTP/1.1");
    assert_eq!(s.body, json!({ "text": "*Northwind* call\n• Send the plan" }));
}

#[test]
fn notion_gets_a_titled_page_under_the_chosen_page() {
    let (base, seen) = server(vec![r#"{"id": "new-page", "url": "https://www.notion.so/Northwind-new"}"#]);
    let at = Endpoints { github: String::new(), linear: String::new(), notion: base };
    let conn = Connection { place: Place::Notion, target: "https://www.notion.so/acme/Sales-calls-0123456789abcdef0123456789abcdef".into() };
    let links = post(&conn, "ntn_x", "Northwind intro call: Sales notes", "## Next steps\n- [ ] You: send the plan", &at).unwrap();
    assert_eq!(links, ["https://www.notion.so/Northwind-new"]);
    let made = seen.recv().unwrap();
    assert_eq!(made.line, "POST /pages HTTP/1.1");
    assert_eq!(made.header("authorization"), "Bearer ntn_x");
    assert_eq!(made.header("notion-version"), "2022-06-28");
    assert_eq!(made.body["parent"], json!({ "page_id": "0123456789abcdef0123456789abcdef" }));
    assert_eq!(made.body["properties"]["title"]["title"][0]["text"]["content"], "Northwind intro call: Sales notes");
    assert_eq!(made.body["children"][1]["type"], "to_do");
}

#[test]
fn test_checks_each_place_without_posting() {
    let (base, seen) = server(vec!["400 no_text", "404 no_service", r#"{"id": "repo"}"#, "404 {}", r#"{"data": {"teams": {"nodes": [{"name": "Engineering"}]}}}"#]);
    let slack = Connection { place: Place::Slack, target: String::new() };
    assert_eq!(check(&slack, &format!("{base}/services/ok"), &Endpoints::default()).unwrap(), "The webhook works.");
    assert!(check(&slack, &format!("{base}/services/gone"), &Endpoints::default()).unwrap_err().contains("no_service"));
    let at = Endpoints { github: base.clone(), linear: format!("{base}/graphql"), notion: base.clone() };
    let github = Connection { place: Place::GitHub, target: "acme/app".into() };
    assert!(check(&github, "t", &at).is_ok());
    let notion = Connection { place: Place::Notion, target: "https://www.notion.so/x-0123456789abcdef0123456789abcdef".into() };
    assert!(check(&notion, "t", &at).unwrap_err().contains("Share it"));
    let linear = Connection { place: Place::Linear, target: "eng".into() };
    assert_eq!(check(&linear, "t", &at).unwrap(), "Team Engineering found.");
    // Slack got an empty message, the rest only reads.
    assert_eq!(seen.recv().unwrap().body, json!({}));
    seen.recv().unwrap();
    assert!(seen.recv().unwrap().line.starts_with("GET /repos/acme/app"));
    assert!(seen.recv().unwrap().line.starts_with("GET /pages/0123456789abcdef0123456789abcdef"));
}

#[test]
fn replies_and_settings_are_checked() {
    assert!(parse_issues("No problems were reported.").is_err());
    assert_eq!(parse_issues("[]").unwrap(), []);
    assert_eq!(To::parse("Issues"), Some(To::Issues));
    assert_eq!(To::parse("slack"), Some(To::Place(Place::Slack)));
    assert_eq!(To::parse("notion"), Some(To::Place(Place::Notion)));
    assert_eq!(To::parse("jira"), None);
    assert!(Place::Slack.check("", "https://example.com/x").is_err());
    assert!(Place::GitHub.check("acme", "t").is_err());
    assert!(Place::GitHub.check("acme/app", "t").is_ok());
    let conns = [Connection { place: Place::Slack, target: String::new() }, Connection { place: Place::Linear, target: "ENG".into() }];
    assert_eq!(To::Issues.resolve(&conns).unwrap().place, Place::Linear);
    assert!(To::Place(Place::GitHub).resolve(&conns).is_none());
    // A tracker is GitHub or Linear, never Slack or Notion.
    assert!(To::Issues.resolve(&[Connection { place: Place::Notion, target: String::new() }]).is_none());
    assert!(Place::Notion.check("https://www.notion.so/acme/Sales-calls", "ntn_x").is_err());
}

/// The real Keychain, with a throwaway item: `cargo test -- --ignored keychain`.
#[test]
#[ignore]
fn keychain_round_trip() {
    let entry = keyring::Entry::new("hex-meetings-test", "round-trip").unwrap();
    entry.set_password("s3cret").unwrap();
    assert_eq!(entry.get_password().unwrap(), "s3cret");
    entry.delete_credential().unwrap();
    assert!(entry.get_password().is_err());
}
