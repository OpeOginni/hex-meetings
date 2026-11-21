//! Whether Hex's speech model fits the language calls are in, and which of
//! Hex's models would. Hex lists every model with its languages
//! (`hex model list --json`); this only compares. It never switches the
//! model: that's Hex's setting, shared with dictation.

use serde::Deserialize;

/// The languages Hex's models list, by ISO 639 code, sorted by name.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("ar", "Arabic"),
    ("bg", "Bulgarian"),
    ("yue", "Cantonese"),
    ("zh", "Chinese"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("et", "Estonian"),
    ("fil", "Filipino"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("de", "German"),
    ("el", "Greek"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("lv", "Latvian"),
    ("lt", "Lithuanian"),
    ("mk", "Macedonian"),
    ("ms", "Malay"),
    ("mt", "Maltese"),
    ("no", "Norwegian"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("es", "Spanish"),
    ("sv", "Swedish"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("vi", "Vietnamese"),
];

pub fn name(code: &str) -> &str {
    LANGUAGES.iter().find(|(c, _)| *c == code).map_or(code, |(_, n)| n)
}

pub fn code(name: &str) -> Option<&'static str> {
    LANGUAGES.iter().find(|(_, n)| *n == name).map(|(c, _)| *c)
}

/// The language of a macOS locale such as `en-DE` or `nb-NO`, if Hex has it.
pub fn from_locale(locale: &str) -> Option<&'static str> {
    let lang = locale.split(['-', '_']).next()?.to_lowercase();
    let lang = if lang == "nb" || lang == "nn" { "no".to_string() } else { lang };
    LANGUAGES.iter().find(|(c, _)| *c == lang).map(|(c, _)| *c)
}

/// The Mac's first preferred language (`defaults read -g AppleLanguages`).
/// Starts a process, so call it off the UI thread.
pub fn mac_language() -> Option<&'static str> {
    let out = std::process::Command::new("defaults").args(["read", "-g", "AppleLanguages"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.split('"').nth(1).and_then(from_locale)
}

/// One model from `hex model list --json`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SpeechModel {
    pub id: String,
    pub name: String,
    /// ISO codes; `*` means any language.
    pub languages: Vec<String>,
    #[serde(default)]
    pub size: String,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub selected: bool,
}

impl SpeechModel {
    pub fn covers(&self, lang: &str) -> bool {
        self.languages.iter().any(|l| l == "*" || l == lang)
    }

    /// "English", "German and English", or "25 languages".
    pub fn languages_label(&self) -> String {
        match self.languages.as_slice() {
            [one] if one == "*" => "any language".into(),
            [one] => name(one).to_string(),
            [a, b] => format!("{} and {}", name(a), name(b)),
            many => format!("{} languages", many.len()),
        }
    }
}

/// What to tell the person about their speech model.
#[derive(Debug, PartialEq)]
pub enum Fit<'a> {
    /// The selected model transcribes the language.
    Fits(&'a SpeechModel),
    /// It doesn't; these do, best first.
    Mismatch { current: &'a SpeechModel, better: Vec<&'a SpeechModel> },
    /// Hex didn't say which model is selected.
    Unknown,
}

/// Compares the selected model with the language. Models made for just that
/// language come first (they're usually the most accurate for it), then the
/// installed ones, then Hex's own order. Hex has no accuracy numbers on its
/// command line, so nothing here ranks models beyond that.
pub fn fit<'a>(models: &'a [SpeechModel], lang: &str) -> Fit<'a> {
    let Some(current) = models.iter().find(|m| m.selected) else { return Fit::Unknown };
    if current.covers(lang) {
        return Fit::Fits(current);
    }
    let mut better: Vec<&SpeechModel> = models.iter().filter(|m| m.covers(lang)).collect();
    better.sort_by_key(|m| (m.languages != [lang], !m.installed));
    Fit::Mismatch { current, better }
}

/// The sentence for a mismatch: "Parakeet Unified English only transcribes
/// English. For German, Hex has Parakeet v3, Nemotron 3.5 and others."
pub fn mismatch_sentence(current: &SpeechModel, better: &[&SpeechModel], lang: &str) -> String {
    let names: Vec<&str> = better.iter().take(2).map(|m| m.name.as_str()).collect();
    let offer = match (names.as_slice(), better.len()) {
        ([], _) => format!("Hex has no model for {} yet.", name(lang)),
        ([one], 1) => format!("For {}, Hex has {one}.", name(lang)),
        ([a, b], 2) => format!("For {}, Hex has {a} and {b}.", name(lang)),
        (names, _) => format!("For {}, Hex has {} and others.", name(lang), names.join(", ")),
    };
    format!("{} only transcribes {}. {offer}", current.name, current.languages_label())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, langs: &[&str], installed: bool, selected: bool) -> SpeechModel {
        SpeechModel {
            id: id.into(),
            name: id.into(),
            languages: langs.iter().map(|l| l.to_string()).collect(),
            size: String::new(),
            installed,
            selected,
        }
    }

    #[test]
    fn locales_map_to_hex_languages() {
        assert_eq!(from_locale("en-DE"), Some("en"));
        assert_eq!(from_locale("nb-NO"), Some("no"));
        assert_eq!(from_locale("zh-Hans-CN"), Some("zh"));
        assert_eq!(from_locale("xx"), None);
        assert_eq!(code("German"), Some("de"));
        assert_eq!(name("de"), "German");
    }

    #[test]
    fn an_english_model_fits_english_and_points_elsewhere_for_german() {
        let models = vec![
            model("english", &["en"], true, true),
            model("multi", &["en", "de", "fr"], false, false),
            model("whisper", &["*"], true, false),
            model("german-only", &["de"], false, false),
        ];
        assert!(matches!(fit(&models, "en"), Fit::Fits(m) if m.id == "english"));
        let Fit::Mismatch { current, better } = fit(&models, "de") else { panic!() };
        assert_eq!(current.id, "english");
        // The German-only model first, then the installed one, then the rest.
        assert_eq!(better.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["german-only", "whisper", "multi"]);
        assert_eq!(
            mismatch_sentence(current, &better, "de"),
            "english only transcribes English. For German, Hex has german-only, whisper and others."
        );
        assert_eq!(fit(&[model("x", &["en"], true, false)], "en"), Fit::Unknown);
    }
}
