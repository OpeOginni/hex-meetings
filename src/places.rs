//! Places Hex Meetings posts a result to, once the person approves it. The
//! agent writes the content (a Slack message, a list of issues) in the
//! format [`To::instructions`] asks for; this module only delivers it.
//! Agents never see the tokens: they live in the macOS Keychain and are
//! read here, on a worker thread, at the moment of posting.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    Slack,
    Notion,
    GitHub,
    Linear,
}

pub const PLACES: [Place; 4] = [Place::Slack, Place::Notion, Place::GitHub, Place::Linear];

/// One thing the person types to connect a place.
pub struct Field {
    pub label: &'static str,
    pub placeholder: &'static str,
}

impl Place {
    pub fn id(self) -> &'static str {
        match self {
            Place::Slack => "slack",
            Place::Notion => "notion",
            Place::GitHub => "github",
            Place::Linear => "linear",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Place::Slack => "Slack",
            Place::Notion => "Notion",
            Place::GitHub => "GitHub",
            Place::Linear => "Linear",
        }
    }

    pub fn from_id(id: &str) -> Option<Place> {
        PLACES.into_iter().find(|p| p.id() == id)
    }

    /// Where results land: a channel name (for your reference), a repository, a team.
    pub fn target_field(self) -> Field {
        match self {
            Place::Slack => Field { label: "Channel", placeholder: "#meeting-notes" },
            Place::Notion => Field { label: "Page", placeholder: "Link to the page reports go under" },
            Place::GitHub => Field { label: "Repository", placeholder: "owner/repo" },
            Place::Linear => Field { label: "Team key", placeholder: "ENG" },
        }
    }

    /// The secret, kept in the Keychain.
    pub fn secret_field(self) -> Field {
        match self {
            Place::Slack => Field { label: "Webhook URL", placeholder: "https://hooks.slack.com/services/…" },
            Place::Notion => Field { label: "Token", placeholder: "ntn_…" },
            Place::GitHub => Field { label: "Token", placeholder: "github_pat_…" },
            Place::Linear => Field { label: "API key", placeholder: "lin_api_…" },
        }
    }

    /// What to create, and where.
    pub fn token_hint(self) -> &'static str {
        match self {
            Place::Slack => "An incoming webhook for the channel",
            Place::Notion => "An internal integration token, with the page shared to it",
            Place::GitHub => "A fine-grained token with Issues read and write on the repository",
            Place::Linear => "A personal API key",
        }
    }

    pub fn token_page(self) -> &'static str {
        match self {
            Place::Slack => "https://api.slack.com/messaging/webhooks",
            Place::Notion => "https://www.notion.so/profile/integrations",
            Place::GitHub => "https://github.com/settings/personal-access-tokens/new",
            Place::Linear => "https://linear.app/settings/account/security",
        }
    }

    /// Catches typos before anything is saved.
    pub fn check(self, target: &str, secret: &str) -> Result<(), String> {
        if secret.is_empty() {
            return Err(format!("Paste the {}.", self.secret_field().label.to_lowercase()));
        }
        match self {
            Place::Slack if !secret.starts_with("https://hooks.slack.com/") => {
                Err("A Slack webhook URL starts with https://hooks.slack.com/.".into())
            }
            Place::Notion if notion::page_id(target).is_none() => Err("Paste the link to a Notion page (Share › Copy link).".into()),
            Place::GitHub if target.split('/').filter(|p| !p.is_empty()).count() != 2 => Err("Write the repository as owner/repo.".into()),
            Place::Linear if target.is_empty() => Err("Write the team's key, such as ENG.".into()),
            _ => Ok(()),
        }
    }
}

/// A connected place: which, and where in it. Its secret is in the Keychain.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Connection {
    pub place: Place,
    pub target: String,
}

impl Connection {
    /// "Slack (#meeting-notes)", "GitHub (acme/app)".
    pub fn label(&self) -> String {
        if self.target.is_empty() { self.place.name().to_string() } else { format!("{} ({})", self.place.name(), self.target) }
    }
}

/// Where a report's result goes: one place, or whichever tracker is connected.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum To {
    Place(Place),
    Issues,
}

impl To {
    pub fn parse(s: &str) -> Option<To> {
        match s.trim().to_lowercase().as_str() {
            "issues" => Some(To::Issues),
            other => Place::from_id(other).map(To::Place),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            To::Place(p) => p.id(),
            To::Issues => "issues",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            To::Place(p) => p.name(),
            To::Issues => "your issue tracker",
        }
    }

    fn makes_issues(self) -> bool {
        matches!(self, To::Issues | To::Place(Place::GitHub | Place::Linear))
    }

    /// The connection this goes to: the place's, or the first connected tracker.
    pub fn resolve(self, connections: &[Connection]) -> Option<&Connection> {
        connections.iter().find(|c| match self {
            To::Place(p) => c.place == p,
            To::Issues => matches!(c.place, Place::GitHub | Place::Linear),
        })
    }

    /// Added to the report's prompt, so the reply is something Hex Meetings can post.
    pub fn instructions(self) -> &'static str {
        if self == To::Place(Place::Notion) {
            return "Reply with only the page's content in Markdown: headings, short paragraphs, lists and to-dos. \
                    No title line (the page is titled for you) and no other text.";
        }
        if self.makes_issues() {
            "Reply with only a JSON array, one object per issue: \
             [{\"title\": \"A short title\", \"body\": \"The details, in Markdown\"}]. \
             No other text. Reply [] if there is nothing to file."
        } else {
            "Reply with only the Slack message, in Slack's formatting (*bold*, _italic_, lines starting with • for lists). \
             No other text."
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Issue {
    pub title: String,
    pub body: String,
}

/// The issues in an agent's reply: the JSON array in it, even inside a code fence.
pub fn parse_issues(output: &str) -> Result<Vec<Issue>, String> {
    #[derive(Deserialize)]
    struct Raw {
        title: String,
        #[serde(default)]
        body: String,
    }
    let (Some(start), Some(end)) = (output.find('['), output.rfind(']')) else {
        return Err("The agent's reply has no list of issues.".into());
    };
    let raw: Vec<Raw> = serde_json::from_str(&output[start..=end]).map_err(|e| format!("The agent's list of issues isn't valid JSON: {e}"))?;
    Ok(raw.into_iter().filter(|r| !r.title.trim().is_empty()).map(|r| Issue { title: r.title.trim().to_string(), body: r.body }).collect())
}

// The Keychain: one item per place, service "hex-meetings".

fn keychain(place: Place) -> Result<keyring::Entry, String> {
    keyring::Entry::new("hex-meetings", place.id()).map_err(|e| e.to_string())
}

pub fn save_secret(place: Place, secret: &str) -> Result<(), String> {
    keychain(place)?.set_password(secret).map_err(|e| format!("Couldn't save to the Keychain: {e}"))
}

pub fn delete_secret(place: Place) {
    if let Ok(entry) = keychain(place) {
        let _ = entry.delete_credential();
    }
}

fn secret(place: Place) -> Result<String, String> {
    keychain(place)?.get_password().map_err(|_| format!("{} isn't connected any more. Connect it again in Setup.", place.name()))
}

/// The real endpoints; tests point these at a local server.
pub struct Endpoints {
    pub github: String,
    pub linear: String,
    pub notion: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints {
            github: "https://api.github.com".into(),
            linear: "https://api.linear.app/graphql".into(),
            notion: "https://api.notion.com/v1".into(),
        }
    }
}

/// Posts the agent's reply and returns links to what was made. `title` names
/// a Notion page. Blocks on the network: call it from a worker thread.
pub fn deliver(connection: &Connection, title: &str, output: &str) -> Result<Vec<String>, String> {
    let secret = secret(connection.place)?;
    post(connection, &secret, title, output, &Endpoints::default())
}

pub fn post(connection: &Connection, secret: &str, title: &str, output: &str, at: &Endpoints) -> Result<Vec<String>, String> {
    match connection.place {
        Place::Notion => {
            let parent = notion::page_id(&connection.target).ok_or("The Notion page link has no page id. Connect Notion again.")?;
            let notion = |req: ureq::Request| req.set("Authorization", &format!("Bearer {secret}")).set("Notion-Version", "2022-06-28");
            let blocks = notion::blocks(output);
            let mut batches = blocks.chunks(notion::BLOCKS_PER_REQUEST);
            let page = send(
                notion(ureq::post(&format!("{}/pages", at.notion))),
                json!({
                    "parent": { "page_id": parent },
                    "properties": { "title": { "title": [{ "text": { "content": title } }] } },
                    "children": batches.next().unwrap_or_default(),
                }),
            )?;
            let id = page["id"].as_str().ok_or("Notion didn't say which page it made.")?.to_string();
            // Notion takes 100 blocks a request; the rest are added in turn.
            for batch in batches {
                send(notion(ureq::patch(&format!("{}/blocks/{id}/children", at.notion))), json!({ "children": batch }))?;
            }
            Ok(page["url"].as_str().map(str::to_string).into_iter().collect())
        }
        Place::Slack => {
            send(ureq::post(secret), json!({ "text": output.trim() }))?;
            Ok(Vec::new())
        }
        Place::GitHub => {
            let url = format!("{}/repos/{}/issues", at.github, connection.target.trim());
            let mut links = Vec::new();
            for issue in parse_issues(output)? {
                let req = ureq::post(&url)
                    .set("Authorization", &format!("Bearer {secret}"))
                    .set("Accept", "application/vnd.github+json")
                    .set("X-GitHub-Api-Version", "2022-11-28");
                let made = send(req, json!({ "title": issue.title, "body": issue.body }))?;
                links.extend(made["html_url"].as_str().map(str::to_string));
            }
            Ok(links)
        }
        Place::Linear => {
            let graphql = |query: &str, variables: Value| -> Result<Value, String> {
                let reply = send(ureq::post(&at.linear).set("Authorization", secret), json!({ "query": query, "variables": variables }))?;
                match reply["errors"][0]["message"].as_str() {
                    Some(e) => Err(format!("Linear: {e}")),
                    None => Ok(reply["data"].clone()),
                }
            };
            let key = connection.target.trim().to_uppercase();
            let teams = graphql("query($key: String!) { teams(filter: { key: { eq: $key } }) { nodes { id } } }", json!({ "key": key }))?;
            let team = teams["teams"]["nodes"][0]["id"].as_str().ok_or(format!("Linear has no team with the key {key}."))?.to_string();
            let mut links = Vec::new();
            for issue in parse_issues(output)? {
                let made = graphql(
                    "mutation($input: IssueCreateInput!) { issueCreate(input: $input) { issue { url } } }",
                    json!({ "input": { "teamId": team, "title": issue.title, "description": issue.body } }),
                )?;
                links.extend(made["issueCreate"]["issue"]["url"].as_str().map(str::to_string));
            }
            Ok(links)
        }
    }
}

/// Checks a connection with its saved secret, posting nothing. Blocks on the
/// network: call it from a worker thread.
pub fn check_connection(connection: &Connection) -> Result<String, String> {
    let secret = secret(connection.place)?;
    check(connection, &secret, &Endpoints::default())
}

/// What "Test" says works, or why it doesn't.
pub fn check(connection: &Connection, secret: &str, at: &Endpoints) -> Result<String, String> {
    let timeout = std::time::Duration::from_secs(20);
    let get = |url: String, auth: String| {
        let mut req = ureq::get(&url).set("User-Agent", "hex-meetings").timeout(timeout);
        if !auth.is_empty() {
            req = req.set("Authorization", &auth);
        }
        req
    };
    let explain = |e: ureq::Error| match e {
        ureq::Error::Status(401, _) | ureq::Error::Status(403, _) => "The token was refused. Make a new one and connect again.".to_string(),
        ureq::Error::Status(404, _) => "Not found, or the token can't see it.".to_string(),
        ureq::Error::Status(code, _) => format!("The server said {code}."),
        e => format!("Couldn't reach the server: {e}"),
    };
    match connection.place {
        // An empty message is never posted: a working webhook answers
        // no_text or invalid_payload, a removed one no_service or invalid_token.
        Place::Slack => match ureq::post(secret).set("User-Agent", "hex-meetings").timeout(timeout).send_json(json!({})) {
            Ok(_) => Ok("The webhook works.".into()),
            Err(ureq::Error::Status(_, res)) => {
                let body = res.into_string().unwrap_or_default();
                if body.contains("no_text") || body.contains("invalid_payload") {
                    Ok("The webhook works.".into())
                } else {
                    Err(format!("Slack refused the webhook ({}). Make a new one and connect again.", body.trim()))
                }
            }
            Err(e) => Err(explain(e)),
        },
        Place::Notion => {
            let page = notion::page_id(&connection.target).ok_or("The page link has no page id.")?;
            get(format!("{}/pages/{page}", at.notion), format!("Bearer {secret}"))
                .set("Notion-Version", "2022-06-28")
                .call()
                .map(|_| "Can reach the page.".to_string())
                .map_err(|e| match e {
                    ureq::Error::Status(404, _) => "Notion can't see that page. Share it with your integration.".to_string(),
                    e => explain(e),
                })
        }
        Place::GitHub => get(format!("{}/repos/{}", at.github, connection.target.trim()), format!("Bearer {secret}"))
            .set("Accept", "application/vnd.github+json")
            .call()
            .map(|_| format!("Can reach {}.", connection.target.trim()))
            .map_err(explain),
        Place::Linear => {
            let key = connection.target.trim().to_uppercase();
            let reply = send(
                ureq::post(&at.linear).set("Authorization", secret),
                json!({ "query": "query($key: String!) { teams(filter: { key: { eq: $key } }) { nodes { name } } }", "variables": { "key": key } }),
            )?;
            if let Some(e) = reply["errors"][0]["message"].as_str() {
                return Err(format!("Linear: {e}"));
            }
            match reply["data"]["teams"]["nodes"][0]["name"].as_str() {
                Some(name) => Ok(format!("Team {name} found.")),
                None => Err(format!("Linear has no team with the key {key}.")),
            }
        }
    }
}

/// Sends JSON and reads the reply, turning HTTP errors into a sentence.
fn send(req: ureq::Request, body: Value) -> Result<Value, String> {
    let req = req.set("User-Agent", "hex-meetings").timeout(std::time::Duration::from_secs(30));
    match req.send_json(body) {
        Ok(res) => {
            let text = res.into_string().map_err(|e| e.to_string())?;
            // Slack's webhook answers plain "ok".
            Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
        }
        Err(ureq::Error::Status(code, res)) => {
            let text = res.into_string().unwrap_or_default();
            let detail = serde_json::from_str::<Value>(&text).ok().and_then(|v| v["message"].as_str().map(str::to_string)).unwrap_or(text);
            Err(format!("The server said {code}: {}", detail.trim()))
        }
        Err(e) => Err(format!("Couldn't reach the server: {e}")),
    }
}

pub mod notion;

#[cfg(test)]
mod tests;
