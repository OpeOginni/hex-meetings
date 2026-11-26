//! Setup: which agent writes the results, where results can go, and how
//! long Hex keeps a call and its audio. The agent and places are this app's
//! settings; the keep times are Hex's own, changed through Hex's command line.

use rune::{El, TextStyle, col, icon, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Menu, MenuItem, SegmentedControl, TextInput};
use rune_theme_opencode::icons as oc;

use super::{Tone, group, header, list_row, logos, section, tag, tile};
use crate::agents::{AGENTS, Agent};
use crate::app::{self, App, HexStatus};
use crate::places::{Connection, PLACES, Place};
use crate::hex;
use crate::paths;

const KEEP: [(&str, &str); 4] = [("day", "A day"), ("week", "A week"), ("month", "A month"), ("forever", "Forever")];
const KEEP_AUDIO: [(&str, &str); 4] = [("day", "A day"), ("week", "A week"), ("month", "A month"), ("none", "With the call")];

fn agent_row(s: &App, k: &Kit, a: &'static Agent, chosen: Option<&str>) -> El {
    let installed = s.installed.contains(&a.id);
    let lead = logos::agent(a.id).map_or_else(|| tile(k, (&oc::CONSOLE, None)), |m| tile(k, m));
    let id = a.id;
    if !installed {
        // Not installed: its official install command, to paste in Terminal.
        let command = a.install;
        let copy = Button::new("Copy")
            .icon(&oc::OUTLINE_COPY)
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Small)
            .help("Copy the install command, then paste it in Terminal")
            .on_press_tx(move |_: &mut App, tx| tx.copy(command.to_string()))
            .build(k)
            .key(format!("copy-install-{id}"));
        let words = col()
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .gap(3.0)
            .child(text(a.name).font(k.ty.ui_medium).color(k.c.text_muted))
            .child(text(command).font(TextStyle { size: 12.0, line_height: 17.0, ..k.ty.mono }).color(k.c.text_faint).wrap_anywhere().selectable(true));
        return row()
            .key(format!("agent-{id}"))
            .w_full()
            .gap(12.0)
            .px(14.0)
            .py(10.0)
            .center_y()
            .child(lead.opacity(0.5))
            .child(words)
            .child(tag(k, "Not installed", Tone::Quiet))
            .child(copy);
    }
    let trailing = if chosen == Some(a.id) {
        tag(k, "Default", Tone::Good)
    } else {
        Button::new("Use")
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Small)
            .on_press(move |s: &mut App| s.set_agent(id))
            .build(k)
            .key(format!("use-{id}"))
    };
    // Only the default agent's model matters here; the rest need no second line.
    let about = (chosen == Some(a.id)).then(|| s.settings.model(a.id).map_or("Its own default model".to_string(), |m| format!("Model: {m}")));
    list_row(k, format!("agent-{id}"), lead, a.name, about, vec![trailing])
}

const TYPE_NAME: &str = "Type a name\u{2026}";

/// A model provider's id as people write it: openai is OpenAI.
fn provider_name(id: &str) -> String {
    match id {
        "openai" => "OpenAI".into(),
        "anthropic" => "Anthropic".into(),
        "github-copilot" => "GitHub Copilot".into(),
        "opencode" => "OpenCode".into(),
        "google" => "Google".into(),
        "xai" => "xAI".into(),
        "openrouter" => "OpenRouter".into(),
        "mistral" => "Mistral".into(),
        "groq" => "Groq".into(),
        "deepseek" => "DeepSeek".into(),
        other => capitalize(&other.replace('-', " ")),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// The default agent's model: a menu of the names it offers when it can say
/// (its own list, or a documented few), else a typed name. Every failure to
/// list falls back to typing, and a saved name the list doesn't have is kept.
fn model_row(s: &App, k: &Kit, layer: &mut Vec<El>) -> Option<El> {
    use crate::agents::Models;
    let a = s.agent()?;
    let typed = TextInput::new(&s.model_edit)
        .placeholder(format!("{}'s default", a.name))
        .width(Some(240.0))
        .on_edit(|s: &mut App| &mut s.model_edit)
        .on_change(|s: &mut App, _| s.save_model())
        .build(k)
        .key("model");
    let (names, about): (Option<Vec<String>>, String) = match &a.models {
        Models::Typed => (None, a.model_hint.to_string()),
        Models::Fixed(list) => (Some(list.iter().map(|m| m.to_string()).collect()), format!("Or type any model name {} accepts.", a.name)),
        Models::Listed(_) => match s.model_lists.get(a.id) {
            Some(Ok(list)) => (Some(list.clone()), format!("{} models from {}. Or type a name.", list.len(), a.name)),
            Some(Err(e)) => (None, format!("{e} Type a name instead.")),
            None => (None, format!("Asking {} for its models\u{2026}", a.name)),
        },
    };
    let current = s.settings.model(a.id).map(str::to_string);
    let controls = match names {
        None => vec![typed],
        Some(names) => {
            // A saved name the list doesn't have stays, as typed.
            let typing = s.model_typing || current.as_ref().is_some_and(|m| !names.contains(m));
            let shown = match (&current, typing) {
                (_, true) => TYPE_NAME.to_string(),
                (None, false) => format!("{}'s default", a.name),
                // A plain name (sonnet) reads as the menu shows it (Sonnet).
                (Some(m), false) if !m.contains('/') => capitalize(m),
                (Some(m), false) => m.clone(),
            };
            let c = &k.c;
            let trigger = row()
                .key("model-trigger")
                .role("button")
                .label(format!("Model: {shown}"))
                .h(28.0)
                .px(10.0)
                .gap(6.0)
                .center_y()
                .radius(6.0)
                .fill(c.bg_base)
                .border(1.0, c.border_weak)
                .hover(c.hover)
                .on_press(|s: &mut App| s.model_menu_open = !s.model_menu_open)
                .child(text(shown).font(k.ty.ui_medium).color(c.text_base).truncate().max_w(260.0))
                .child(icon(&oc::OUTLINE_CHEVRON_DOWN, 14.0).color(c.icon_muted));
            let mut menu = Menu::new("model-menu")
                .trigger(trigger)
                .open(s.model_menu_open)
                .min_width(340.0)
                .placement(rune_kit::Placement::BottomEnd)
                .on_close(|s: &mut App| s.model_menu_open = false)
                .item(MenuItem::new(format!("{}'s default", a.name)).checked(current.is_none() && !typing).on_select(|s: &mut App| s.set_model(None)))
                .item(MenuItem::new(TYPE_NAME).checked(typing).on_select(|s: &mut App| {
                    s.model_typing = true;
                    s.model_menu_open = false;
                }))
                .separator();
            // Several providers (OpenCode, Pi): a heading per provider, each
            // row the model's own name. One provider (Claude Code): a plain list.
            let mut provider = None;
            for n in names {
                let (group, label) = match n.split_once('/') {
                    Some((p, m)) => (Some(p.to_string()), m.to_string()),
                    None => (None, capitalize(&n)),
                };
                if group.is_some() && group != provider {
                    menu = menu.group_label(provider_name(group.as_deref().unwrap_or_default()));
                    provider = group;
                }
                let checked = !typing && current.as_deref() == Some(n.as_str());
                menu = menu.item(MenuItem::new(label).checked(checked).on_select(move |s: &mut App| s.set_model(Some(&n))));
            }
            let menu = menu.build(k, layer);
            if typing { vec![menu, typed] } else { vec![menu] }
        }
    };
    Some(list_row(k, "model-row", tile(k, (&oc::MODELS, None)), &format!("Model for {}", a.name), Some(about), controls))
}

/// The reports whose results go to a place, for its row.
fn used_by(s: &App, goes_there: impl Fn(&crate::reports::Report) -> bool) -> Option<String> {
    match s.reports.iter().filter(|r| goes_there(r)).count() {
        0 => None,
        1 => Some("1 report sends here".into()),
        n => Some(format!("{n} reports send here")),
    }
}

fn small(k: &Kit, label: &str, key: String, variant: ButtonVariant, f: impl Fn(&mut App) + 'static) -> El {
    Button::new(label.to_string()).variant(variant).control_size(ControlSize::Small).on_press(f).build(k).key(key)
}

fn c_target(c: Option<&Connection>) -> Option<String> {
    c.map(|c| c.target.clone()).filter(|t| !t.is_empty())
}

/// A place Hex Meetings posts to: connected, or a button to connect it.
fn place_row(s: &App, k: &Kit, place: Place) -> El {
    let id = place.id();
    let connected = s.settings.connections.iter().find(|c| c.place == place);
    let used = used_by(s, |r| r.to.is_some_and(|to| to.resolve(std::slice::from_ref(&Connection { place, target: String::new() })).is_some()));
    // A Notion page link is long; the row says the place is connected and how much uses it.
    let target = if place == Place::Notion { None } else { Some(c_target(connected)).flatten() };
    // A Test result replaces the line under the name until the next one.
    let check = s.checks.get(&place);
    let (about, trailing) = match connected {
        Some(_) => (
            match check {
                Some(Ok(ok)) => format!("\u{2713} {ok}"),
                Some(Err(e)) => format!("\u{26a0} {e}"),
                None => [target, used].into_iter().flatten().collect::<Vec<_>>().join(", "),
            },
            vec![
                match check {
                    Some(Err(_)) => tag(k, "Not working", Tone::Bad),
                    _ => tag(k, "Connected", Tone::Good),
                },
                small(k, if s.checking == Some(place) { "Testing\u{2026}" } else { "Test" }, format!("test-{id}"), ButtonVariant::Neutral, move |s| s.test_connection(place)),
                small(k, "Disconnect", format!("disconnect-{id}"), ButtonVariant::Neutral, move |s| s.disconnect(place)),
            ],
        ),
        None if s.connecting == Some(place) => (place.token_hint().to_string(), vec![]),
        None => (
            place.token_hint().to_string(),
            vec![small(k, "Connect", format!("connect-{id}"), ButtonVariant::Neutral, move |s| s.start_connect(place))],
        ),
    };
    let r = list_row(k, format!("place-{id}"), tile(k, logos::for_place(place)), place.name(), Some(about), trailing);
    if s.connecting != Some(place) {
        return r;
    }
    // The connect form, under its row.
    let (target, secret) = (place.target_field(), place.secret_field());
    let field = |label: &str, input: El| {
        row()
            .w_full()
            .gap(12.0)
            .center_y()
            .child(text(label.to_string()).font(k.ty.small_medium).color(k.c.text_muted).w(96.0))
            .child(col().grow(1.0).shrink().min_w(0.0).child(input))
    };
    let form = col()
        .key(format!("connect-form-{id}"))
        .w_full()
        .gap(8.0)
        .pl(58.0)
        .pr(14.0)
        .pb(14.0)
        .child(field(
            target.label,
            TextInput::new(&s.form_target).placeholder(target.placeholder).width(None).on_edit(|s: &mut App| &mut s.form_target).build(k).key("form-target"),
        ))
        .child(field(
            secret.label,
            TextInput::new(&s.form_secret)
                .placeholder(secret.placeholder)
                .secure()
                .width(None)
                .on_edit(|s: &mut App| &mut s.form_secret)
                .on_submit(|s: &mut App, _| s.save_connection())
                .build(k)
                .key("form-secret"),
        ))
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(text("Kept in your Mac's Keychain. Agents never see it.").font(k.ty.small).color(k.c.text_faint).grow(1.0).shrink().min_w(0.0))
                .child(small(k, "Get one", format!("token-page-{id}"), ButtonVariant::Neutral, move |_| app::reveal(place.token_page())))
                .child(small(k, "Cancel", "cancel-connect".into(), ButtonVariant::Neutral, |s| s.connecting = None))
                .child(small(k, "Save", "save-connect".into(), ButtonVariant::Submit, |s| s.save_connection())),
        );
    col().w_full().child(r).child(form)
}

fn places(s: &App, k: &Kit, hair: f32) -> El {
    let folder = crate::paths::summaries_dir();
    let open = folder.clone();
    let markdown = list_row(
        k,
        "place-markdown",
        tile(k, logos::markdown()),
        "Markdown files",
        Some(format!("Save a result from its card, into {}", paths::tilde(&folder))),
        vec![show_in_finder(k, "open-folder", folder.exists().then_some(open))],
    );
    let mut rows = vec![markdown];
    rows.extend(PLACES.into_iter().map(|p| place_row(s, k, p)));
    // Anywhere else, the agent posts with its own connections: a report's
    // "Goes to" can be "Your agent's own tools".
    let agent = s.agent().map_or("your agent".to_string(), |a| a.name.to_string());
    let about = [
        Some(format!("Jira, or anything {agent} is connected to. Pick \u{201c}Your agent's tools\u{201d} in a report")),
        used_by(s, |r| r.sends_to.is_some()),
    ];
    rows.push(list_row(
        k,
        "place-agent",
        tile(k, s.agent().and_then(|a| logos::agent(a.id)).unwrap_or((&oc::CONSOLE, None))),
        "Through your agent",
        Some(about.into_iter().flatten().collect::<Vec<_>>().join(". ")),
        vec![],
    ));
    group(k, "places", hair, rows)
}

/// How long Hex keeps finished calls, as a row with its own control.
fn keeping(s: &App, k: &Kit) -> El {
    let current = s.overview.as_ref().and_then(|o| o.as_ref().ok()).and_then(|o| o.retention());
    let control = keep_control(k, "Finished calls", "meetings.retention", &KEEP, current);
    list_row(k, "keep-row", tile(k, (&oc::ARCHIVE, None)), "Finished calls", Some(status(s, hex::retention_sentence(current))), vec![control])
}

/// When Hex trashes a finished call's audio, keeping its transcript.
fn keeping_audio(s: &App, k: &Kit) -> El {
    // Unset means the same as `none`: the audio goes with the call.
    let current = s.overview.as_ref().and_then(|o| o.as_ref().ok()).map(|o| o.audio_retention().unwrap_or("none"));
    let control = keep_control(k, "Call audio", "meetings.audio_retention", &KEEP_AUDIO, current);
    list_row(k, "audio-row", tile(k, (&oc::OUTLINE_AUDIO, None)), "Call audio", Some(status(s, hex::audio_sentence(current))), vec![control])
}

/// The row's line: Hex's setting, or why it can't be read yet.
fn status(s: &App, sentence: &str) -> String {
    match &s.overview {
        Some(Ok(_)) => sentence.to_string(),
        Some(Err(_)) => "Open Hex to see how long it keeps calls".into(),
        None => "Reading Hex's setting\u{2026}".into(),
    }
}

/// Picks one of `choices` for a Hex keep setting.
fn keep_control(k: &Kit, label: &str, setting: &'static str, choices: &'static [(&'static str, &'static str)], current: Option<&str>) -> El {
    let selected = choices.iter().position(|(v, _)| Some(*v) == current);
    let labels: Vec<&str> = choices.iter().map(|(_, l)| *l).collect();
    SegmentedControl::new(&labels, selected)
        .label(label)
        .fit()
        .on_pick(move |s: &mut App, i, _| s.set_keep(setting, choices[i].0))
        .build(k)
        .key(setting)
}

/// "Show in Finder" with the folder icon; nothing until the folder exists.
fn show_in_finder(k: &Kit, key: &str, folder: Option<std::path::PathBuf>) -> El {
    let Some(folder) = folder else { return rune::el() };
    Button::new("Show in Finder")
        .icon(&oc::FOLDER)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .on_press(move |_: &mut App| app::reveal(&folder))
        .build(k)
        .key(key.to_string())
}

/// Hex's own meeting settings in one list, each with a way to change it.
fn hex_settings(s: &App, k: &Kit, hair: f32, layer: &mut Vec<El>) -> El {
    let c = &k.c;
    if !matches!(s.hex, Some(HexStatus::Found { .. })) {
        let download = Button::new("Download Hex")
            .variant(ButtonVariant::Neutral)
            .control_size(ControlSize::Small)
            .on_press(|_: &mut App| app::reveal(hex::DOWNLOAD_PAGE))
            .build(k)
            .key("download-hex");
        return group(k, "hex", hair, vec![list_row(k, "hex-missing", tile(k, (&oc::WAVEFORM, None)), "Hex isn't installed", Some("Its meeting settings show here once it is.".into()), vec![download])]);
    }
    let small = |label: &str, key: &str, f: fn(&mut App)| {
        Button::new(label.to_string()).variant(ButtonVariant::Neutral).control_size(ControlSize::Small).on_press(f).build(k).key(key.to_string())
    };

    let meetings = {
        let (about, trailing) = match s.meetings_on {
            Some(true) => ("Hex offers to record your calls", tag(k, "On", Tone::Good)),
            Some(false) => ("Off, so Hex doesn't record calls", small("Turn on", "turn-on", |s| s.turn_on_meetings())),
            None => ("Hex isn't running, so it can't say", small("Turn on", "turn-on", |s| s.turn_on_meetings())),
        };
        list_row(k, "hex-meetings", tile(k, (&oc::WAVEFORM, None)), "Meetings", Some(about.into()), vec![trailing])
    };
    let language = list_row(
        k,
        "hex-language",
        tile(k, (&oc::GLOBE, None)),
        "Language of your calls",
        Some("Checked against Hex's speech model".into()),
        vec![super::language_select(s, k, layer)],
    );
    let (fits, about) = super::speech_status(s);
    let speech = list_row(
        k,
        "hex-model",
        tile(k, if fits { (&oc::OUTLINE_AUDIO, None) } else { (&oc::WARNING, Some(c.warning.icon)) }),
        "Speech model",
        Some(about),
        vec![small("Choose in Hex", "choose-model", |s| s.open_in_hex(true))],
    );
    group(k, "hex", hair, vec![meetings, language, speech, keeping(s, k), keeping_audio(s, k)])
}

/// Light, dark, or following the Mac.
fn appearance(s: &App, k: &Kit, hair: f32) -> El {
    const MODES: [(Option<&str>, &str); 3] = [(None, "System"), (Some("light"), "Light"), (Some("dark"), "Dark")];
    let current = MODES.iter().position(|(v, _)| *v == s.settings.appearance.as_deref());
    let labels: Vec<&str> = MODES.iter().map(|(_, l)| *l).collect();
    let control = SegmentedControl::new(&labels, current.or(Some(0)))
        .label("Appearance")
        .fit()
        .on_pick(|s: &mut App, i, _| s.set_appearance(MODES[i].0))
        .build(k)
        .key("appearance");
    group(k, "app", hair, vec![list_row(k, "appearance-row", tile(k, (&oc::APPEARANCE, None)), "Appearance", None, vec![control])])
}

pub fn page(s: &App, k: &Kit, hair: f32, layer: &mut Vec<El>) -> El {
    let chosen = s.agent().map(|a| a.id);
    // Installed agents first; the rest below.
    let (installed, missing): (Vec<_>, Vec<_>) = AGENTS.iter().partition(|a| s.installed.contains(&a.id));
    let agents = group(k, "agents", hair, installed.into_iter().chain(missing).map(|a| agent_row(s, k, a, chosen)).collect());
    let mut agent_part = col().w_full().gap(12.0).child(agents);
    if let Some(m) = model_row(s, k, layer) {
        agent_part = agent_part.child(group(k, "model", hair, vec![m]));
    }
    let check_again = Button::new("Check again")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .help("Look for agents installed since the app opened")
        .on_press(|s: &mut App| s.refresh_agents())
        .build(k)
        .key("check-agents");
    let open_hex = Button::new("Open in Hex")
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .disabled(!matches!(s.hex, Some(HexStatus::Found { .. })))
        .on_press(|s: &mut App| s.open_in_hex(false))
        .build(k)
        .key("open-hex");
    let head = header(k, "Setup", "Your agent, where results go, and Hex's meeting settings.", Vec::new());
    let mut content = col().w_full().gap(28.0);
    if let Some(n) = &s.notice {
        content = content.child(super::notice(k, n, Some(|s: &mut App| s.notice = None)));
    }
    // A speech model that can't transcribe the calls' language is the one
    // setting that quietly ruins every result, so it's said up top.
    let (fits, about_speech) = super::speech_status(s);
    if !fits && matches!(s.overview, Some(Ok(_))) {
        content = content.child(
            row()
                .key("speech-warning")
                .w_full()
                .gap(10.0)
                .px(14.0)
                .py(10.0)
                .center_y()
                .radius(k.radius.lg)
                .fill(k.c.warning.bg)
                .child(rune::icon(&oc::WARNING, 16.0).color(k.c.warning.icon))
                .child(text(about_speech).font(k.ty.small.lh(18.0)).color(k.c.text_base).wrap().grow(1.0).shrink().min_w(0.0))
                .child(
                    Button::new("Choose in Hex")
                        .variant(ButtonVariant::Neutral)
                        .control_size(ControlSize::Small)
                        .on_press(|s: &mut App| s.open_in_hex(true))
                        .build(k)
                        .key("speech-warning-choose"),
                ),
        );
    }
    content = content
        .child(section(
            k,
            "setup-agent",
            "Agent",
            Some("Writes every report, with its own model."),
            vec![check_again],
            agent_part,
        ))
        .child(section(k, "setup-places", "Places", Some("Connect a place once; results post after you approve them."), vec![], places(s, k, hair)))
        .child(section(
            k,
            "setup-hex",
            "Hex",
            Some("Hex's own settings."),
            vec![open_hex],
            hex_settings(s, k, hair, layer),
        ))
        .child(section(k, "setup-app", "App", None, vec![], appearance(s, k, hair)));
    super::page_shell("setup", head, content)
}
