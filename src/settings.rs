//! The person's choices, kept in `settings.json` in the data folder: the
//! default agent, where results are saved as Markdown, and the calls they
//! already decided to keep or delete after a summary.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Settings {
    /// An agent id from [`crate::agents::AGENTS`]; none means the first installed.
    #[serde(default)]
    pub agent: Option<String>,
    /// Meeting ids whose "keep or delete" question is answered.
    #[serde(default)]
    pub settled: Vec<String>,
    /// The model each agent runs with, by agent id; none uses the agent's own default.
    #[serde(default)]
    pub models: std::collections::BTreeMap<String, String>,
    /// The language calls are in (ISO code); none follows the Mac's language.
    #[serde(default)]
    pub language: Option<String>,
    /// "light" or "dark"; none follows the Mac.
    #[serde(default)]
    pub appearance: Option<String>,
    /// Places Hex Meetings posts to. Their secrets are in the Keychain.
    #[serde(default)]
    pub connections: Vec<crate::places::Connection>,
}

impl Settings {
    pub fn load(file: &Path) -> Settings {
        std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> Result<(), String> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(file, json).map_err(|e| format!("Could not save {}: {e}", file.display()))
    }

    pub fn model(&self, agent_id: &str) -> Option<&str> {
        self.models.get(agent_id).map(|m| m.trim()).filter(|m| !m.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_a_missing_file_is_the_default() {
        let file = std::env::temp_dir().join(format!("hex-meetings-settings-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&file);
        assert_eq!(Settings::load(&file), Settings::default());
        let mut s = Settings { agent: Some("claude".into()), settled: vec!["m1".into()], ..Settings::default() };
        s.models.insert("claude".into(), "opus".into());
        s.save(&file).unwrap();
        assert_eq!(Settings::load(&file), s);
        let _ = std::fs::remove_file(&file);
    }
}
