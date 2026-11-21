//! The app's state and the work that happens off the UI thread. Workers
//! push [`Event`]s into an inbox and wake the window; `poll` drains them on
//! the UI thread. Nothing in a handler blocks on Hex or an agent.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rune::{TextEdit, Tx, Waker};

use crate::activity::{self, Outcome, Run};
use crate::hex::{self, Detail, Meeting};
use crate::recipes::{self, Recipe, Vars};
use crate::{paths, runner};

/// How often the meetings list is read again while the app runs. A read
/// takes a few milliseconds and never touches the recording side of Hex.
const REFRESH: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Meetings,
    Recipes,
    Activity,
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
    Tick,
}

pub struct Running {
    pub meeting_id: String,
    pub recipe: String,
    pub started: Instant,
    pub stop: Arc<AtomicBool>,
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
    pub recipes: Vec<Recipe>,
    pub recipe_errors: Vec<String>,
    /// The recipe waiting for the person to confirm where the call goes.
    pub armed: Option<String>,
    pub running: Option<Running>,
    pub activity: Vec<Run>,
    /// The activity entry whose output is open.
    pub open_run: Option<i64>,
    pub notice: Option<String>,
    pub inbox: Arc<Mutex<Vec<Event>>>,
    pub waker: Option<Waker>,
    /// Scenes and tests: no workers, no files.
    pub offline: bool,
}

impl App {
    pub fn new() -> App {
        let (recipes, recipe_errors) = recipes::load(&paths::recipes_dir());
        App { recipes, recipe_errors, activity: activity::load(&paths::activity_file()), ..App::default() }
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
        let send = self.send();
        std::thread::spawn(move || {
            let Some(path) = hex::find_binary() else {
                send(Event::Hex(HexStatus::Missing));
                return;
            };
            send(Event::Hex(HexStatus::Found { path: path.clone(), version: hex::version(&path) }));
            let mut last = None;
            loop {
                let result = hex::list(&path);
                if last.as_ref() != Some(&result) {
                    last = Some(result.clone());
                    send(Event::Meetings(result));
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
        changed
    }

    fn apply(&mut self, event: Event, tx: &mut Tx) {
        match event {
            Event::Hex(status) => self.hex = Some(status),
            Event::Meetings(Ok(meetings)) => {
                let first_load = !self.meetings_loaded;
                self.meetings = meetings;
                self.meetings_loaded = true;
                self.list_error = None;
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

    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }

    /// Runs the armed recipe on the selected meeting's transcript.
    pub fn run_armed(&mut self) {
        let Some(recipe) = self.armed.take().and_then(|id| self.recipe(&id).cloned()) else { return };
        let Some(detail) = self.detail.clone() else { return };
        if self.running.is_some() {
            return;
        }
        let m = &detail.meeting;
        let transcript = hex::transcript_text(&detail.entries);
        let date = crate::time::date_time(m.timestamp_ms);
        let duration = hex::duration(m.duration_ms);
        let summary = detail.notes.as_ref().map(|n| n.summary.clone()).unwrap_or_default();
        let argv = recipe.argv(&Vars {
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
            recipe: recipe.name.clone(),
            started: Instant::now(),
            stop: stop.clone(),
        });
        let send = self.send();
        let base = Run {
            at_ms: chrono::Utc::now().timestamp_millis(),
            meeting_id: m.id.clone(),
            meeting_title: m.title.clone(),
            recipe: recipe.name.clone(),
            sends_to: recipe.sends_to.clone(),
            program: recipe.command.first().cloned().unwrap_or_default(),
            outcome: Outcome::Failed,
            output: String::new(),
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

    pub fn reload_recipes(&mut self) {
        let (recipes, errors) = recipes::load(&paths::recipes_dir());
        self.recipes = recipes;
        self.recipe_errors = errors;
        if self.armed.as_ref().is_some_and(|id| !self.recipes.iter().any(|r| &r.id == id)) {
            self.armed = None;
        }
    }

    pub fn clear_activity(&mut self) {
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

/// Opens a folder in Finder (or the desktop's file manager).
pub fn reveal(path: &std::path::Path) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(path).spawn();
}
