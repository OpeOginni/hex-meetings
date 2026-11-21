//! The app's state and the work that happens off the UI thread. Workers
//! push [`Event`]s into an inbox and wake the window; `poll` drains them on
//! the UI thread. Nothing in a handler blocks on Hex or an agent.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rune::{TextEdit, Tx, Waker};

use crate::activity::{self, Outcome, Run};
use crate::agents::{self, Agent};
use crate::hex::{self, Detail, Meeting};
use crate::places::{self, Connection, Place, To};
use crate::reports::{self, Plan, Report, Vars};
use crate::settings::Settings;
use crate::{paths, runner};

/// How often the meetings list is read again while the app runs. A read
/// takes a few milliseconds and never touches the recording side of Hex.
const REFRESH: Duration = Duration::from_secs(15);

/// How often a missing Hex is looked for again, so installing it while the
/// app is open is noticed.
const LOOK_AGAIN: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Meetings,
    Reports,
    Activity,
    Setup,
    About,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HexStatus {
    Looking,
    Found { path: PathBuf, version: Option<String> },
    Missing,
}

pub enum Event {
    Hex(HexStatus),
    Meetings(Result<Vec<Meeting>, String>),
    Detail(String, Result<Detail, String>),
    RunEnded(Run),
    Overview(Result<hex::Overview, String>),
    MacLanguage(Option<&'static str>),
    /// An agent's model list, or why it couldn't be read.
    ModelList(&'static str, Result<Vec<String>, String>),
    /// A run's result was posted, or failed to be.
    Delivered(i64, activity::Delivery),
    /// A finished call's transcript, for a report that runs by itself.
    AutoDetail(String, Result<Detail, String>),
    /// What a place's Test found.
    Checked(Place, Result<String, String>),
    /// A place's secret is in the Keychain; save the connection.
    Connected(Connection),
    /// Whether Labs › Meetings is on; None while Hex can't say (not running).
    MeetingsOn(Option<bool>),
    Notice(String),
    Tick,
}

pub struct Running {
    pub meeting_id: String,
    pub report: String,
    /// The agent writing it, for its logo; None until it starts.
    pub agent: Option<&'static str>,
    pub started: Instant,
    pub stop: Arc<AtomicBool>,
}

/// The Summary as set on the open call: which agent writes it, where it
/// goes, and anything to focus on. For that call only; a call starts with
/// the defaults.
#[derive(Default)]
pub struct Quick {
    pub agent: Option<&'static Agent>,
    pub to: Option<To>,
    pub focus: TextEdit,
    /// Which menu is open: "to" or "agent".
    pub menu: Option<&'static str>,
}

/// A report as it's being edited: the four fields the editor shows, and the
/// file's other settings, kept as they were.
pub struct Draft {
    /// The file it saves to; None for a new report.
    pub id: Option<String>,
    pub name: TextEdit,
    pub focus: TextEdit,
    pub to: Option<To>,
    /// None uses the default agent.
    pub agent: Option<&'static Agent>,
    /// Set when the agent posts with its own tools; `place` says where.
    pub sends_to: Option<String>,
    pub place: TextEdit,
    pub auto: bool,
    /// Kept from the file.
    pub description: String,
    /// Which menu is open: "to" or "agent".
    pub menu: Option<&'static str>,
    pub confirm_delete: bool,
}

#[derive(Default)]
pub struct App {
    pub page: Page,
    pub hex: Option<HexStatus>,
    pub meetings: Vec<Meeting>,
    pub meetings_loaded: bool,
    pub list_error: Option<String>,
    pub search: TextEdit,
    pub selected: Option<String>,
    pub detail: Option<Detail>,
    pub detail_error: Option<String>,
    pub reports: Vec<Report>,
    pub report_errors: Vec<String>,
    /// The report waiting for the person to confirm where the call goes.
    pub armed: Option<String>,
    /// The open call's Summary settings.
    pub quick: Quick,
    pub running: Option<Running>,
    pub activity: Vec<Run>,
    /// The activity entry whose output is open.
    pub open_run: Option<i64>,
    pub notice: Option<String>,
    pub settings: Settings,
    /// Ids of the agents found on this computer.
    pub installed: Vec<&'static str>,
    /// How long Hex keeps finished calls, once read.
    pub overview: Option<Result<hex::Overview, String>>,
    /// The Mac's language, the default for the language calls are in.
    pub mac_language: Option<&'static str>,
    /// The language menu is open.
    pub language_open: bool,
    /// Finished calls already seen; None until the first list, which only
    /// sets the baseline, so calls from before the app opened don't run.
    pub finished: Option<std::collections::HashSet<String>>,
    /// Automatic runs waiting for the one before them: (call, report id).
    pub auto_queue: std::collections::VecDeque<(String, String)>,
    /// The last Test of each connected place, and the one being tested.
    pub checks: std::collections::HashMap<Place, Result<String, String>>,
    pub checking: Option<Place>,
    /// Clear was pressed once; the next press clears Activity.
    pub confirm_clear: bool,
    /// The report being written or edited on the Reports page.
    pub editing: Option<Draft>,
    /// The meeting whose deletion was handed to Hex's window.
    pub deleting: Option<String>,
    /// The last result saved as Markdown: its run and the file.
    pub saved: Option<(i64, PathBuf)>,
    /// Whether Hex records calls (Labs › Meetings); None until Hex answers.
    pub meetings_on: Option<bool>,
    /// "Turn on Meetings" is waiting for Hex.
    pub turning_on: bool,
    /// The default agent's model, as typed in Setup.
    pub model_edit: TextEdit,
    /// Each listing agent's models, once its list command answered (or failed).
    pub model_lists: std::collections::HashMap<&'static str, Result<Vec<String>, String>>,
    pub model_menu_open: bool,
    /// "Type a name…" was picked from the model menu.
    pub model_typing: bool,
    /// The run whose result is being posted.
    pub posting: Option<i64>,
    /// The place whose connect form is open, and what's typed in it.
    pub connecting: Option<Place>,
    pub form_target: TextEdit,
    pub form_secret: TextEdit,
    pub inbox: Arc<Mutex<Vec<Event>>>,
    pub waker: Option<Waker>,
    /// Scenes and tests: no workers, no files.
    pub offline: bool,
}

impl App {
    pub fn new() -> App {
        let (reports, report_errors) = reports::load(&paths::reports_dir());
        let mut app = App {
            reports,
            report_errors,
            activity: activity::load(&paths::activity_file()),
            settings: Settings::load(&paths::settings_file()),
            ..App::default()
        };
        app.refresh_agents();
        app
    }

    /// Looks for the agents again (after installing one) and shows the
    /// default agent's model in Setup.
    pub fn refresh_agents(&mut self) {
        if !self.offline {
            self.installed = agents::AGENTS.iter().filter(|a| a.installed()).map(|a| a.id).collect();
        }
        let model = self.agent().and_then(|a| self.settings.model(a.id)).unwrap_or_default().to_string();
        self.model_edit = TextEdit::new(model);
        self.model_typing = false;
        self.model_menu_open = false;
        // Ask the agent for its models again, in the background.
        if let Some(agent) = self.agent().filter(|a| matches!(a.models, agents::Models::Listed(_)) && !self.offline) {
            let send = self.send();
            std::thread::spawn(move || send(Event::ModelList(agent.id, agent.list_models())));
        }
    }

    /// Picks the default agent's model; None uses the agent's own default.
    pub fn set_model(&mut self, model: Option<&str>) {
        self.model_edit = TextEdit::new(model.unwrap_or_default());
        self.model_typing = false;
        self.model_menu_open = false;
        self.save_model();
    }

    pub fn go(&mut self, page: Page) {
        self.page = page;
        if page == Page::Setup {
            self.refresh_agents();
            self.refresh_overview();
        }
        // Reports edited by hand show up without a Reload button.
        if page == Page::Reports && !self.offline {
            self.reload_reports();
        }
        if page == Page::About {
            self.refresh_overview();
        }
    }

    fn send(&self) -> impl Fn(Event) + Send + 'static {
        let inbox = self.inbox.clone();
        let waker = self.waker.clone();
        move |event| {
            if let Ok(mut q) = inbox.lock() {
                q.push(event);
            }
            if let Some(w) = &waker {
                w();
            }
        }
    }

    pub fn hex_path(&self) -> Option<PathBuf> {
        match &self.hex {
            Some(HexStatus::Found { path, .. }) => Some(path.clone()),
            _ => None,
        }
    }

    /// Finds Hex, then keeps the meetings list current.
    pub fn start(&mut self, waker: Waker) {
        self.waker = Some(waker);
        if self.offline {
            return;
        }
        self.hex = Some(HexStatus::Looking);
        let lang = self.send();
        std::thread::spawn(move || lang(Event::MacLanguage(crate::speech::mac_language())));
        let send = self.send();
        std::thread::spawn(move || {
            let mut told = false;
            let path = loop {
                if let Some(path) = hex::find_binary() {
                    break path;
                }
                if !told {
                    send(Event::Hex(HexStatus::Missing));
                    told = true;
                }
                std::thread::sleep(LOOK_AGAIN);
            };
            send(Event::Hex(HexStatus::Found { path: path.clone(), version: hex::version(&path) }));
            send(Event::Overview(hex::overview(&path)));
            let (mut last, mut last_on) = (None, None);
            loop {
                let result = hex::list(&path);
                if last.as_ref() != Some(&result) {
                    last = Some(result.clone());
                    send(Event::Meetings(result));
                }
                let on = Some(hex::meetings_enabled(&path).ok());
                if on != last_on {
                    last_on = on;
                    send(Event::MeetingsOn(on.flatten()));
                }
                std::thread::sleep(REFRESH);
            }
        });
    }

    /// Lands worker results. True when anything visible changed.
    pub fn poll(&mut self, tx: &mut Tx) -> bool {
        let events: Vec<Event> = match self.inbox.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(_) => return false,
        };
        let changed = !events.is_empty();
        for event in events {
            self.apply(event, tx);
        }
        self.next_automatic();
        changed
    }

    fn apply(&mut self, event: Event, tx: &mut Tx) {
        match event {
            Event::Hex(status) => self.hex = Some(status),
            Event::Meetings(Ok(meetings)) => {
                self.queue_automatic(&meetings);
                let first_load = !self.meetings_loaded;
                self.meetings = meetings;
                self.meetings_loaded = true;
                self.list_error = None;
                if self.deleting.as_ref().is_some_and(|id| !self.meetings.iter().any(|m| &m.id == id)) {
                    self.deleting = None;
                }
                if first_load {
                    tx.instant();
                }
                match &self.selected {
                    // A meeting still being written gets its newest transcript.
                    Some(id) if self.meetings.iter().any(|m| &m.id == id && m.in_progress) => {
                        let id = id.clone();
                        self.load_detail(&id);
                    }
                    Some(id) if !self.meetings.iter().any(|m| &m.id == id) => self.select(None),
                    Some(_) => {}
                    None => {
                        let first = self.meetings.first().map(|m| m.id.clone());
                        self.select(first);
                    }
                }
            }
            Event::Meetings(Err(e)) => {
                self.meetings_loaded = true;
                self.list_error = Some(e);
            }
            Event::Detail(id, result) => {
                if self.selected.as_deref() == Some(id.as_str()) {
                    match result {
                        Ok(detail) => {
                            self.detail = Some(detail);
                            self.detail_error = None;
                        }
                        Err(e) => self.detail_error = Some(e),
                    }
                }
            }
            Event::RunEnded(run) => {
                if let Err(e) = activity::append(&paths::activity_file(), &run) {
                    self.notice = Some(format!("The run finished but couldn't be saved to Activity: {e}"));
                }
                self.activity.insert(0, run);
                self.running = None;
            }
            Event::Overview(o) => self.overview = Some(o),
            Event::MacLanguage(l) => self.mac_language = l,
            Event::MeetingsOn(on) => {
                self.meetings_on = on;
                self.turning_on = false;
            }
            Event::ModelList(id, list) => {
                self.model_lists.insert(id, list);
            }
            Event::Delivered(at, delivery) => {
                self.posting = None;
                if let Some(run) = self.activity.iter_mut().find(|r| r.at_ms == at) {
                    run.delivery = Some(delivery);
                }
                self.save_activity();
            }
            // The placeholder from next_automatic gives way to the real run, or to nothing.
            Event::AutoDetail(report_id, result) => {
                self.running = None;
                match (result, self.report(&report_id).cloned()) {
                    (Ok(detail), Some(report)) => self.start_run(detail, report, true),
                    (Err(e), _) => self.notice = Some(format!("A report couldn't run by itself: {e}")),
                    (Ok(_), None) => {}
                }
            }
            Event::Checked(place, result) => {
                self.checks.insert(place, result);
                self.checking = None;
            }
            Event::Connected(c) => {
                self.checks.remove(&c.place);
                self.settings.connections.retain(|x| x.place != c.place);
                self.settings.connections.push(c);
                self.connecting = None;
                self.save_settings();
            }
            Event::Notice(n) => {
                self.notice = Some(n);
                self.turning_on = false;
            }
            Event::Tick => {}
        }
    }

    pub fn select(&mut self, id: Option<String>) {
        if self.selected == id {
            return;
        }
        self.selected = id.clone();
        self.detail = None;
        self.detail_error = None;
        self.armed = None;
        self.quick = Quick::default();
        if let Some(id) = id {
            self.load_detail(&id);
        }
    }

    fn load_detail(&mut self, id: &str) {
        let Some(path) = self.hex_path() else { return };
        let send = self.send();
        let id = id.to_string();
        std::thread::spawn(move || {
            let result = hex::show(&path, &id);
            send(Event::Detail(id, result));
        });
    }

    pub fn selected_meeting(&self) -> Option<&Meeting> {
        let id = self.selected.as_deref()?;
        self.meetings.iter().find(|m| m.id == id)
    }

    /// Runs a failed or stopped run's report again: opens its call with the
    /// report ready to confirm, so where the transcript goes is shown first.
    pub fn retry(&mut self, at_ms: i64) {
        let Some(run) = self.activity.iter().find(|r| r.at_ms == at_ms) else { return };
        let (meeting, report) = (run.meeting_id.clone(), run.report.clone());
        if !self.meetings.iter().any(|m| m.id == meeting) {
            self.notice = Some("That call isn't in Hex any more.".into());
            return;
        }
        // The Summary isn't a saved report: set it up as that run had it.
        if report == reports::builtin().name {
            let (agent, to) = (agents::AGENTS.iter().find(|a| a.program == run.program), run.to.as_deref().and_then(To::parse));
            self.go(Page::Meetings);
            self.select(Some(meeting));
            self.quick.agent = agent;
            self.quick.to = to;
            return;
        }
        let Some(id) = self.report_by_name(&report).map(|r| r.id.clone()) else {
            self.notice = Some(format!("The report \u{201c}{report}\u{201d} doesn't exist any more."));
            return;
        };
        self.go(Page::Meetings);
        self.select(Some(meeting));
        self.armed = Some(id);
    }

    /// A report by its name, as a run records it.
    pub fn report_by_name(&self, name: &str) -> Option<&Report> {
        std::iter::once(reports::builtin()).chain(&self.reports).find(|r| r.name == name)
    }

    /// A report by id: the built-in Summary, or one of the person's.
    pub fn report(&self, id: &str) -> Option<&Report> {
        if id == reports::BUILTIN_ID {
            return Some(reports::builtin());
        }
        self.reports.iter().find(|r| r.id == id)
    }

    /// The agent reports use unless they name another.
    pub fn agent(&self) -> Option<&'static Agent> {
        agents::default_agent(self.settings.agent.as_deref(), &self.installed)
    }

    pub fn plan(&self, report: &Report) -> Result<Plan, String> {
        report.plan(self.agent(), |id| self.settings.model(id).map(str::to_string))
    }

    fn save_settings(&mut self) {
        if self.offline {
            return;
        }
        if let Err(e) = self.settings.save(&paths::settings_file()) {
            self.notice = Some(e);
        }
    }

    pub fn set_agent(&mut self, id: &str) {
        self.settings.agent = Some(id.to_string());
        self.save_settings();
        self.refresh_agents();
    }

    /// Saves the model typed for the default agent; empty uses its own default.
    pub fn save_model(&mut self) {
        let Some(agent) = self.agent() else { return };
        let model = self.model_edit.text.trim().to_string();
        if model.is_empty() {
            self.settings.models.remove(agent.id);
        } else {
            self.settings.models.insert(agent.id.to_string(), model);
        }
        self.save_settings();
    }

    /// Opens Hex if needed and turns on Labs › Meetings, then reads it back.
    pub fn turn_on_meetings(&mut self) {
        let Some(path) = self.hex_path().filter(|_| !self.offline) else { return };
        self.turning_on = true;
        let send = self.send();
        std::thread::spawn(move || match hex::turn_on_meetings(&path) {
            Ok(()) => {
                send(Event::MeetingsOn(hex::meetings_enabled(&path).ok()));
                send(Event::Overview(hex::overview(&path)));
            }
            Err(e) => send(Event::Notice(format!("Couldn't turn on Meetings in Hex: {e}"))),
        });
    }

    /// Reads Hex's meeting settings and models again (Setup shows them).
    pub fn refresh_overview(&mut self) {
        let Some(path) = self.hex_path().filter(|_| !self.offline) else { return };
        let send = self.send();
        std::thread::spawn(move || send(Event::Overview(hex::overview(&path))));
    }

    pub fn connection(&self, to: To) -> Option<&Connection> {
        to.resolve(&self.settings.connections)
    }

    fn save_activity(&mut self) {
        if self.offline {
            return;
        }
        if let Err(e) = activity::save_all(&paths::activity_file(), &self.activity) {
            self.notice = Some(e);
        }
    }

    /// Posts a finished run's result to its connected place.
    pub fn post_result(&mut self, at_ms: i64) {
        let Some(run) = self.activity.iter().find(|r| r.at_ms == at_ms) else { return };
        let Some(conn) = run.to.as_deref().and_then(To::parse).and_then(|to| self.connection(to)).cloned() else { return };
        if self.offline || self.posting.is_some() {
            return;
        }
        self.posting = Some(at_ms);
        let output = run.output.clone();
        let title = format!("{}: {}", run.meeting_title, run.report);
        let send = self.send();
        std::thread::spawn(move || {
            let delivery = match places::deliver(&conn, &title, &output) {
                Ok(links) => activity::Delivery::Posted { to: conn.label(), links },
                Err(error) => activity::Delivery::Failed { to: conn.label(), error },
            };
            send(Event::Delivered(at_ms, delivery));
        });
    }

    /// Keeps a result here without posting it.
    pub fn skip_post(&mut self, at_ms: i64) {
        if let Some(run) = self.activity.iter_mut().find(|r| r.at_ms == at_ms) {
            run.delivery = Some(activity::Delivery::Skipped);
        }
        self.save_activity();
    }

    /// Tries a failed post again.
    pub fn retry_post(&mut self, at_ms: i64) {
        if let Some(run) = self.activity.iter_mut().find(|r| r.at_ms == at_ms) {
            run.delivery = None;
        }
        self.post_result(at_ms);
    }

    pub fn start_connect(&mut self, place: Place) {
        self.connecting = Some(place);
        let target = self.settings.connections.iter().find(|c| c.place == place).map(|c| c.target.clone()).unwrap_or_default();
        self.form_target = TextEdit::new(target);
        self.form_secret = TextEdit::new("");
    }

    /// Checks the form, puts the secret in the Keychain (off the UI thread:
    /// macOS may ask first), then saves the connection.
    pub fn save_connection(&mut self) {
        let Some(place) = self.connecting else { return };
        let target = self.form_target.text.trim().to_string();
        let secret = self.form_secret.text.trim().to_string();
        if let Err(e) = place.check(&target, &secret) {
            self.notice = Some(e);
            return;
        }
        self.form_secret = TextEdit::new("");
        let connection = Connection { place, target };
        if self.offline {
            return self.apply_offline(connection);
        }
        let send = self.send();
        std::thread::spawn(move || match places::save_secret(place, &secret) {
            Ok(()) => send(Event::Connected(connection)),
            Err(e) => send(Event::Notice(e)),
        });
    }

    fn apply_offline(&mut self, connection: Connection) {
        self.settings.connections.retain(|x| x.place != connection.place);
        self.settings.connections.push(connection);
        self.connecting = None;
    }

    /// Checks a connected place with its saved secret, posting nothing.
    pub fn test_connection(&mut self, place: Place) {
        let Some(conn) = self.settings.connections.iter().find(|c| c.place == place).cloned() else { return };
        if self.offline || self.checking.is_some() {
            return;
        }
        self.checking = Some(place);
        let send = self.send();
        std::thread::spawn(move || send(Event::Checked(place, places::check_connection(&conn))));
    }

    pub fn disconnect(&mut self, place: Place) {
        self.checks.remove(&place);
        self.settings.connections.retain(|c| c.place != place);
        self.save_settings();
        if !self.offline {
            std::thread::spawn(move || places::delete_secret(place));
        }
    }

    /// The language calls are in: the one chosen, else the Mac's, else English.
    pub fn language(&self) -> &str {
        self.settings.language.as_deref().or(self.mac_language).unwrap_or("en")
    }

    pub fn set_language(&mut self, code: &str) {
        self.settings.language = Some(code.to_string());
        self.language_open = false;
        self.save_settings();
    }

    /// Opens Hex at a pane: its speech model picker, or its Meetings settings.
    pub fn open_in_hex(&mut self, models: bool) {
        let Some(path) = self.hex_path().filter(|_| !self.offline) else { return };
        let send = self.send();
        std::thread::spawn(move || {
            let opened = if models { hex::open_models(&path) } else { hex::open_meetings(&path) };
            if let Err(e) = opened {
                send(Event::Notice(format!("Couldn't open Hex: {e}")));
            }
        });
    }

    /// "light", "dark", or None to follow the Mac.
    pub fn set_appearance(&mut self, appearance: Option<&str>) {
        self.settings.appearance = appearance.map(str::to_string);
        self.save_settings();
    }

    /// Saves a run's result as a Markdown file in the summaries folder.
    pub fn save_result(&mut self, at_ms: i64) {
        let Some(run) = self.activity.iter().find(|r| r.at_ms == at_ms) else { return };
        if self.offline {
            return;
        }
        match run.save_markdown(&paths::summaries_dir()) {
            Ok(path) => self.saved = Some((at_ms, path)),
            Err(e) => self.notice = Some(e),
        }
    }

    pub fn is_settled(&self, meeting_id: &str) -> bool {
        self.settings.settled.iter().any(|id| id == meeting_id)
    }

    /// Keeps the call in Hex, which removes it when its own time is up.
    pub fn keep_call(&mut self, meeting_id: &str) {
        if self.deleting.as_deref() == Some(meeting_id) {
            self.deleting = None;
        }
        if !self.is_settled(meeting_id) {
            self.settings.settled.push(meeting_id.to_string());
            self.save_settings();
        }
    }

    /// Opens Hex's Meetings pane so the person can delete the call there.
    pub fn delete_in_hex(&mut self, meeting_id: &str) {
        self.deleting = Some(meeting_id.to_string());
        let Some(path) = self.hex_path().filter(|_| !self.offline) else { return };
        let send = self.send();
        std::thread::spawn(move || {
            if let Err(e) = hex::open_meetings(&path) {
                send(Event::Notice(format!("Couldn't open Hex: {e}")));
            }
        });
    }

    /// Changes how long Hex keeps finished calls, then reads it back.
    pub fn set_retention(&mut self, value: &'static str) {
        let Some(path) = self.hex_path().filter(|_| !self.offline) else { return };
        let send = self.send();
        std::thread::spawn(move || {
            if let Err(e) = hex::set_retention(&path, value) {
                send(Event::Notice(format!("Hex didn't change how long it keeps calls: {e}")));
            }
            send(Event::Overview(hex::overview(&path)));
        });
    }

    /// The built-in Summary with the open call's settings.
    pub fn quick_report(&self) -> Report {
        let mut report = reports::builtin().clone();
        report.agent = self.quick.agent;
        report.to = self.quick.to;
        let extra = self.quick.focus.text.trim();
        if !extra.is_empty() {
            report.focus = format!("{}\n\nFor this call, also: {extra}", report.focus);
        }
        report
    }

    /// Runs the Summary on the open call, as set on its page.
    pub fn summarize(&mut self) {
        let Some(detail) = self.detail.clone() else { return };
        if self.running.is_some() {
            return;
        }
        self.quick.menu = None;
        let report = self.quick_report();
        self.start_run(detail, report, false);
    }

    /// Runs the armed report on the selected meeting's transcript.
    pub fn run_armed(&mut self) {
        let Some(report) = self.armed.take().and_then(|id| self.report(&id).cloned()) else { return };
        let Some(detail) = self.detail.clone() else { return };
        if self.running.is_some() {
            return;
        }
        self.start_run(detail, report, false);
    }

    /// Queues each automatic report for calls that have just finished.
    fn queue_automatic(&mut self, meetings: &[Meeting]) {
        let done: std::collections::HashSet<String> =
            meetings.iter().filter(|m| m.status == "complete" && !m.in_progress).map(|m| m.id.clone()).collect();
        let Some(seen) = &mut self.finished else {
            self.finished = Some(done);
            return;
        };
        let new: Vec<String> = done.iter().filter(|id| !seen.contains(*id)).cloned().collect();
        seen.extend(done);
        for meeting in new {
            for r in self.reports.iter().filter(|r| r.auto) {
                self.auto_queue.push_back((meeting.clone(), r.id.clone()));
            }
        }
    }

    /// Starts the next automatic run once nothing else is running.
    fn next_automatic(&mut self) {
        if self.running.is_some() || self.offline {
            return;
        }
        let Some((meeting, report)) = self.auto_queue.pop_front() else { return };
        let Some(path) = self.hex_path() else { return };
        let Some(name) = self.report(&report).map(|r| r.name.clone()) else { return };
        // Marks the app busy until the transcript is read and the run starts.
        self.running = Some(Running { meeting_id: meeting.clone(), report: name, agent: None, started: Instant::now(), stop: Default::default() });
        let send = self.send();
        std::thread::spawn(move || send(Event::AutoDetail(report, hex::show(&path, &meeting))));
    }

    /// Runs a report on a call's transcript, by the person or by itself.
    fn start_run(&mut self, detail: Detail, report: Report, auto: bool) {
        let plan = match self.plan(&report) {
            Ok(plan) => plan,
            Err(e) => {
                self.notice = Some(e);
                return;
            }
        };
        let m = &detail.meeting;
        let transcript = hex::transcript_text(&detail.entries);
        let date = crate::time::date_time(m.timestamp_ms);
        let duration = hex::duration(m.duration_ms);
        let summary = detail.notes.as_ref().map(|n| n.summary.clone()).unwrap_or_default();
        let argv = report.argv(&plan.command, &Vars {
            id: &m.id,
            title: &m.title,
            date: &date,
            duration: &duration,
            app: m.application.as_deref().unwrap_or("a call"),
            summary: &summary,
            transcript: &transcript,
        });
        let stop = Arc::new(AtomicBool::new(false));
        self.running = Some(Running {
            meeting_id: m.id.clone(),
            report: report.name.clone(),
            agent: plan.agent,
            started: Instant::now(),
            stop: stop.clone(),
        });
        let send = self.send();
        let base = Run {
            at_ms: chrono::Utc::now().timestamp_millis(),
            meeting_id: m.id.clone(),
            meeting_title: m.title.clone(),
            report: report.name.clone(),
            sends_to: plan.sends_to.clone(),
            program: plan.command[0].clone(),
            outcome: Outcome::Failed,
            output: String::new(),
            to: report.to.map(|t| t.id().to_string()),
            delivery: None,
            auto,
        };
        let done = Arc::new(AtomicBool::new(false));
        let ticker_done = done.clone();
        let tick = self.send();
        // The elapsed time on the running strip moves once a second.
        std::thread::spawn(move || {
            while !ticker_done.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
                tick(Event::Tick);
            }
        });
        std::thread::spawn(move || {
            let ended = runner::run(&argv, &stop);
            done.store(true, Ordering::Relaxed);
            send(Event::RunEnded(Run { outcome: ended.outcome, output: activity::truncate(&ended.output), ..base }));
        });
    }

    pub fn stop_run(&mut self) {
        if let Some(r) = &self.running {
            r.stop.store(true, Ordering::Relaxed);
        }
    }

    pub fn new_report(&mut self) {
        self.editing = Some(Draft {
            id: None,
            name: TextEdit::new(""),
            focus: TextEdit::new(""),
            to: None,
            agent: None,
            sends_to: None,
            auto: false,
            place: TextEdit::new(""),
            description: String::new(),
            menu: None,
            confirm_delete: false,
        });
    }

    pub fn edit_report(&mut self, id: &str) {
        let Some(r) = self.report(id).cloned() else { return };
        self.editing = Some(Draft {
            id: Some(r.id),
            name: TextEdit::new(r.name),
            focus: TextEdit::new(r.focus),
            to: r.to,
            agent: r.agent,
            place: TextEdit::new(r.sends_to.clone().unwrap_or_default()),
            auto: r.auto,
            sends_to: r.sends_to,
            description: r.description,
            menu: None,
            confirm_delete: false,
        });
    }

    /// Turns the open report into a new one, named as a copy.
    pub fn duplicate_report(&mut self) {
        if let Some(d) = &mut self.editing {
            d.id = None;
            d.name = TextEdit::new(format!("{} copy", d.name.text.trim()));
            d.confirm_delete = false;
        }
    }

    /// Writes the open report to its file (a new one for a new report).
    pub fn save_report(&mut self) {
        let Some(d) = &self.editing else { return };
        let name = d.name.text.trim().to_string();
        if name.is_empty() {
            self.notice = Some("Give the report a name.".into());
            return;
        }
        let dir = paths::reports_dir();
        let id = d.id.clone().unwrap_or_else(|| reports::new_id(&dir, &name));
        let report = Report {
            path: dir.join(format!("{id}.md")),
            id,
            name,
            description: d.description.clone(),
            agent: d.agent,
            sends_to: d.sends_to.as_ref().map(|_| {
                let place = d.place.text.trim();
                if place.is_empty() { "where the focus says".to_string() } else { place.to_string() }
            }),
            to: d.to,
            // The agent posting by itself is never automatic.
            auto: d.auto && d.sends_to.is_none(),
            focus: d.focus.text.trim().to_string(),
        };
        if self.offline {
            self.reports.retain(|r| r.id != report.id);
            self.reports.push(report);
            self.editing = None;
            return;
        }
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&report.path, report.to_markdown())) {
            self.notice = Some(format!("Couldn't save {}: {e}", report.path.display()));
            return;
        }
        self.editing = None;
        self.reload_reports();
    }

    /// Deletes the open report's file, after a second press.
    pub fn delete_report(&mut self) {
        let Some(d) = &mut self.editing else { return };
        if !d.confirm_delete {
            d.confirm_delete = true;
            return;
        }
        if let Some(id) = d.id.clone() {
            if !self.offline {
                let _ = std::fs::remove_file(paths::reports_dir().join(format!("{id}.md")));
            }
            self.reports.retain(|r| r.id != id);
        }
        self.editing = None;
        if !self.offline {
            self.reload_reports();
        }
    }

    pub fn reload_reports(&mut self) {
        let (reports, errors) = reports::load(&paths::reports_dir());
        self.reports = reports;
        self.report_errors = errors;
        if self.armed.as_ref().is_some_and(|id| id != reports::BUILTIN_ID && !self.reports.iter().any(|r| &r.id == id)) {
            self.armed = None;
        }
    }

    /// Clears the log, after a second press.
    pub fn clear_activity(&mut self) {
        if !self.confirm_clear {
            self.confirm_clear = true;
            return;
        }
        self.confirm_clear = false;
        match activity::clear(&paths::activity_file()) {
            Ok(()) => {
                self.activity.clear();
                self.open_run = None;
            }
            Err(e) => self.notice = Some(e),
        }
    }

    /// The meetings whose title or app contains the search text.
    pub fn visible_meetings(&self) -> Vec<&Meeting> {
        let q = self.search.text.trim().to_lowercase();
        self.meetings
            .iter()
            .filter(|m| {
                q.is_empty()
                    || m.title.to_lowercase().contains(&q)
                    || m.application.as_deref().unwrap_or_default().to_lowercase().contains(&q)
            })
            .collect()
    }

    /// The newest run for a meeting, shown on its page.
    pub fn last_run(&self, meeting_id: &str) -> Option<&Run> {
        self.activity.iter().find(|r| r.meeting_id == meeting_id)
    }
}

/// Opens a folder in Finder, or a link in the browser.
pub fn reveal(path: impl AsRef<std::ffi::OsStr>) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(path).spawn();
}

/// Shows a file selected in Finder (or opens its folder elsewhere).
pub fn reveal_file(path: &std::path::Path) {
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
    } else if let Some(dir) = path.parent() {
        reveal(dir);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_calls_that_finish_while_the_app_runs_get_automatic_reports() {
        let mut app = crate::demo::app();
        let auto_ids: Vec<String> = app.reports.iter().filter(|r| r.auto).map(|r| r.id.clone()).collect();
        assert!(!auto_ids.is_empty(), "the demo has an automatic report");
        let mut list = crate::demo::meetings();
        // The first list is the baseline: calls already finished don't run.
        app.queue_automatic(&list);
        assert!(app.auto_queue.is_empty());
        // A call still recording doesn't run yet.
        let mut new = list[0].clone();
        new.id = "new-call".into();
        new.status = "recording".into();
        new.in_progress = true;
        list.push(new.clone());
        app.queue_automatic(&list);
        assert!(app.auto_queue.is_empty());
        // Once it finishes, each automatic report is queued for it, once.
        let last = list.len() - 1;
        list[last].status = "complete".into();
        list[last].in_progress = false;
        app.queue_automatic(&list);
        app.queue_automatic(&list);
        let queued: Vec<_> = app.auto_queue.iter().cloned().collect();
        assert_eq!(queued, auto_ids.iter().map(|id| ("new-call".to_string(), id.clone())).collect::<Vec<_>>());
    }
}
