//! Reports: what to make from a call. The list shows each report's route
//! (agent → where it goes); a press opens the editor, which has the four
//! things a report is: a name, what to focus on, where it goes, which agent.
//! Each saves as a Markdown file, so they can be edited by hand too.

use rune::{El, col, el, icon, row, spacer, text};
use rune_kit::{Button, ButtonVariant, Kit, Menu, MenuItem, Switch, TextArea, TextInput};
use rune_theme_opencode::icons as oc;

use super::{group, header, logos, tile};
use crate::agents::AGENTS;
use crate::app::{self, App, Draft};
use crate::paths;
use crate::places::{PLACES, To};
use crate::reports::Report;

/// A report's logo: where its result goes. One that stays in the app shows
/// the waveform, the Meetings icon, since the result appears on the call's page.
pub fn report_mark(s: &App, r: &Report) -> logos::Mark {
    if let Some(to) = r.to {
        return match (to, s.connection(to)) {
            (_, Some(c)) => logos::for_place(c.place),
            (To::Place(p), None) => logos::for_place(p),
            (To::Issues, None) => (&oc::CHECKLIST, None),
        };
    }
    r.sends_to.as_deref().and_then(logos::place).unwrap_or((&oc::WAVEFORM, None))
}

/// "[agent] OpenCode → [place] your Notion": the route a report sends a call along.
fn route(s: &App, k: &Kit, r: &Report) -> El {
    let c = &k.c;
    let Ok(plan) = s.plan(r) else {
        return text(s.plan(r).unwrap_err()).font(k.ty.small).color(c.danger.fg).wrap();
    };
    let step = |logo: Option<El>, label: String| row().gap(5.0).center_y().children(logo).child(text(label).font(k.ty.small).color(c.text_muted));
    let dest = match (r.to, &r.sends_to) {
        (Some(to), _) => match s.connection(to) {
            Some(c) => step(Some(super::mark(k, logos::for_place(c.place), 12.0)), c.label()),
            None => step(None, format!("{}, once connected in Setup", to.name())),
        },
        (None, Some(place)) => step(r.sends_to.as_deref().and_then(logos::place).map(|m| super::mark(k, m, 12.0)), format!("{place}, by the agent")),
        (None, None) => step(Some(icon(&oc::WAVEFORM, 12.0).color(c.icon_muted)), "On the call".into()),
    };
    row()
        .gap(8.0)
        .center_y()
        .flex_wrap()
        .child(step(plan.agent.and_then(|id| super::agent_logo(k, id, 12.0)), plan.runs.clone()))
        .child(icon(&oc::ARROW_RIGHT, 12.0).color(c.icon_faint))
        .child(dest)
        .children(r.auto.then(|| super::tag(k, "When a call ends", super::Tone::Busy)))
}

fn report_row(s: &App, k: &Kit, r: &Report) -> El {
    let c = &k.c;
    let command = s.plan(r).map(|p| crate::reports::command_line(&p.command)).unwrap_or_default();
    let id = r.id.clone();
    row()
        .key(format!("report-card-{}", r.id))
        .role("button")
        .label(format!("Edit {}", r.name))
        .w_full()
        .gap(12.0)
        .px(14.0)
        .py(12.0)
        .center_y()
        .hover(c.hover)
        .help(r.blurb())
        .on_press(move |s: &mut App| s.edit_report(&id))
        .child(tile(k, report_mark(s, r)))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(4.0)
                .child(text(r.name.clone()).font(k.ty.ui_medium).color(c.text_base))
                .child(route(s, k, r).help(command)),
        )
        .child(icon(&oc::OUTLINE_CHEVRON_RIGHT, 14.0).color(c.icon_faint))
}

const AGENT_TOOLS: &str = "Your agent's tools";

/// What "Goes to" says it's set to.
fn destination_label(s: &App, d: &Draft) -> String {
    match d.to {
        Some(To::Issues) => s.connection(To::Issues).map_or("Your tracker".to_string(), |c| format!("Your tracker ({})", c.place.name())),
        Some(To::Place(p)) => p.name().to_string(),
        None if d.sends_to.is_some() => AGENT_TOOLS.to_string(),
        None => "On the call".to_string(),
    }
}

fn pick_to(s: &mut App, to: Option<To>, agent_tools: bool) {
    let Some(d) = s.editing.as_mut() else { return };
    d.menu = None;
    d.to = to;
    if agent_tools {
        d.sends_to.get_or_insert_with(String::new);
    } else {
        d.sends_to = None;
    }
}

fn pick_agent(s: &mut App, agent: Option<&'static crate::agents::Agent>) {
    let Some(d) = s.editing.as_mut() else { return };
    d.menu = None;
    d.agent = agent;
}

/// A menu's button: the chosen item's logo and name, and a chevron.
pub(super) fn menu_trigger(k: &Kit, key: &str, logo: logos::Mark, label: String, toggle: fn(&mut App)) -> El {
    let c = &k.c;
    row()
        .key(format!("{key}-trigger"))
        .role("button")
        .label(label.clone())
        .h(30.0)
        .pl(8.0)
        .pr(10.0)
        .gap(8.0)
        .center_y()
        .radius(6.0)
        .fill(c.bg_base)
        .border(1.0, c.border_weak)
        .hover(c.hover)
        .on_press(toggle)
        .child(super::mark(k, logo, 16.0))
        .child(text(label).font(k.ty.ui_medium).color(c.text_base))
        .child(icon(&oc::OUTLINE_CHEVRON_DOWN, 14.0).color(c.icon_muted))
}

/// A labelled row of the form; `fill` stretches the control (text fields),
/// otherwise it keeps its own width (menus, so they open under themselves).
fn field(k: &Kit, label: &str, fill: bool, control: El) -> El {
    let label = text(label.to_string()).font(k.ty.ui_medium).color(k.c.text_base).w(96.0).pt(if fill { 8.0 } else { 4.0 });
    let control = if fill { col().grow(1.0).shrink().min_w(0.0).child(control) } else { col().gap(4.0).child(control) };
    row().w_full().gap(16.0).items_start().px(14.0).py(12.0).child(label).child(control)
}

fn editor(s: &App, k: &Kit, d: &Draft, hair: f32, layer: &mut Vec<El>) -> (El, El) {
    let c = &k.c;
    let name = TextInput::new(&d.name)
        .placeholder("Customer feedback")
        .width(None)
        .on_edit(|s: &mut App| &mut s.editing.as_mut().expect("editing").name)
        .build(k)
        .key("report-name");
    let focus = TextArea::new(&d.focus)
        .placeholder("Feature requests and complaints. Quote the customer. Skip pricing talk.")
        .width(None)
        .auto_grow(5, 14)
        .on_edit(|s: &mut App| &mut s.editing.as_mut().expect("editing").focus)
        .build(k)
        .key("report-focus");

    // Only a report that already uses it: "whichever tracker is connected".
    let tracker = (d.to == Some(To::Issues)).then(|| {
        let label = s.connection(To::Issues).map_or("Your tracker".to_string(), |c| format!("Your tracker ({})", c.place.name()));
        let logo = s.connection(To::Issues).map_or((&oc::CHECKLIST, None), |c| logos::for_place(c.place));
        MenuItem::new(label).lead(lead(k, logo)).description("Whichever of GitHub or Linear is connected").checked(true)
    });
    let to_menu = places_menu(s, k, "report-to", destination_mark(s, d), destination_label(s, d), d.to, d.sends_to.is_none(), tracker, d.menu == Some("to"), |s| toggle(s, "to"), close, |s, to| {
        pick_to(s, to, false)
    });
    // The agent posting with its own tools is for anywhere not built in, so
    // it sits last, beside connecting a place.
    let to_menu = to_menu
        .item(
            MenuItem::new(AGENT_TOOLS)
                .lead(lead(k, agent_mark(s, d)))
                .description("For anywhere else, with its own connections")
                .checked(d.sends_to.is_some())
                .on_select(|s: &mut App| pick_to(s, None, true)),
        )
        .item(connect_item(k))
        .build(k, layer);
    // Said only when it matters: a place that isn't connected, or an agent
    // that will use its tools unasked.
    let warning = match d.to {
        Some(To::Issues) if s.connection(To::Issues).is_none() => Some("No tracker is connected. Connect GitHub or Linear in Setup.".to_string()),
        Some(to) if s.connection(to).is_none() => Some(format!("{} isn't connected. Connect it in Setup.", to.name())),
        None if d.sends_to.is_some() => Some("Your agent posts it with its own connections, using its tools without asking.".to_string()),
        _ => None,
    };
    let goes_to = col().gap(6.0).child(to_menu).children(warning.map(|w| {
        row()
            .gap(6.0)
            .center_y()
            .pl(2.0)
            .child(icon(&oc::WARNING, 13.0).color(c.warning.icon))
            .child(text(w).font(k.ty.small).color(c.text_muted).wrap().shrink().min_w(0.0))
    }));
    let place = d.sends_to.is_some().then(|| {
        TextInput::new(&d.place)
            .placeholder("Our Sales Notion, or the #support channel")
            .width(None)
            .on_edit(|s: &mut App| &mut s.editing.as_mut().expect("editing").place)
            .build(k)
            .key("report-place")
    });
    let agent = agent_menu(s, k, "report-agent", d.agent, d.menu == Some("agent"), |s| toggle(s, "agent"), close, pick_agent).build(k, layer);

    let form = group(
        k,
        "report-form",
        hair,
        vec![
            field(k, "Name", true, name),
            field(k, "Focus on", true, focus),
            field(k, "Agent", false, agent),
            field(k, "Goes to", false, goes_to),
        ]
        .into_iter()
        .chain(place.map(|p| field(k, "Where", true, p)))
        .chain([field(k, "Runs", false, auto_switch(k, d))])
        .collect(),
    );
    let small = |label: &str, key: &str, variant: ButtonVariant, f: fn(&mut App)| {
        Button::new(label.to_string()).variant(variant).on_press(f).build(k).key(key.to_string())
    };
    let mut footer = row().w_full().gap(8.0).center_y();
    if d.id.is_some() {
        footer = footer
            .child(super::destructive_button(k, if d.confirm_delete { "Delete for good" } else { "Delete" }, "delete-report", App::delete_report))
            .child(small("Duplicate", "duplicate-report", ButtonVariant::Neutral, App::duplicate_report));
    }
    footer = footer
        .child(spacer())
        .child(small("Cancel", "cancel-report", ButtonVariant::Neutral, |s| s.editing = None))
        .child(small("Save", "save-report", ButtonVariant::Submit, App::save_report));
    let title = if d.id.is_some() { "Edit report" } else { "New report" };
    let head = header(k, title, "Say what to look for. The app adds the rest.", vec![]);
    let mut out = col().w_full().gap(20.0);
    if let Some(n) = &s.notice {
        out = out.child(super::notice(k, n, Some(|s: &mut App| s.notice = None)));
    }
    if d.confirm_delete {
        out = out.child(text("Press Delete for good to remove this report. This can't be undone.").font(k.ty.small).color(c.danger.fg));
    }
    (head, out.child(form).child(footer))
}

fn lead(k: &Kit, m: logos::Mark) -> El {
    el().size(18.0, 18.0).center().child(super::mark(k, m, 16.0))
}

/// Goes to Setup, where places are connected.
pub(super) fn connect_item(k: &Kit) -> MenuItem {
    MenuItem::new("Connect a place in Setup\u{2026}").lead(lead(k, (&oc::SLIDERS, None))).on_select(|s: &mut App| {
        close(s);
        s.quick.menu = None;
        s.go(crate::app::Page::Setup);
    })
}

/// Where a result goes: "On the call", then each connected place with its
/// logo, then `extra`, then the places without a key, greyed until they're
/// connected in Setup. Ends with a separator.
#[allow(clippy::too_many_arguments)]
pub(super) fn places_menu(
    s: &App,
    k: &Kit,
    key: &str,
    mark: logos::Mark,
    label: String,
    to: Option<To>,
    on_call: bool,
    extra: Option<MenuItem>,
    open: bool,
    toggle: fn(&mut App),
    close: fn(&mut App),
    pick: fn(&mut App, Option<To>),
) -> Menu {
    let mut menu = Menu::new(key.to_string())
        .trigger(menu_trigger(k, key, mark, label, toggle))
        .open(open)
        .min_width(280.0)
        .placement(rune_kit::Placement::BottomStart)
        .on_close(move |s: &mut App| close(s))
        .item(
            MenuItem::new("On the call")
                .lead(lead(k, (&oc::WAVEFORM, None)))
                .description("Shown here, to copy or save")
                .checked(to.is_none() && on_call)
                .on_select(move |s: &mut App| pick(s, None)),
        );
    let (connected, missing): (Vec<_>, Vec<_>) = PLACES.into_iter().partition(|p| s.connection(To::Place(*p)).is_some());
    for p in connected {
        let target = s.connection(To::Place(p)).map(|c| c.target.clone()).filter(|t| !t.is_empty() && p != crate::places::Place::Notion);
        let mut item = MenuItem::new(p.name())
            .lead(lead(k, logos::for_place(p)))
            .checked(to == Some(To::Place(p)))
            .on_select(move |s: &mut App| pick(s, Some(To::Place(p))));
        if let Some(t) = target {
            item = item.description(t);
        }
        menu = menu.item(item);
    }
    if let Some(item) = extra {
        menu = menu.item(item);
    }
    if !missing.is_empty() {
        menu = menu.separator();
        for p in missing {
            menu = menu.item(MenuItem::new(p.name()).lead(lead(k, logos::for_place(p))).description("Not connected").disabled(true));
        }
    }
    menu.separator()
}

/// Which agent writes it: the default, then the installed agents, then the
/// rest, greyed.
#[allow(clippy::too_many_arguments)]
pub(super) fn agent_menu(
    s: &App,
    k: &Kit,
    key: &str,
    chosen: Option<&'static crate::agents::Agent>,
    open: bool,
    toggle: fn(&mut App),
    close: fn(&mut App),
    pick: fn(&mut App, Option<&'static crate::agents::Agent>),
) -> Menu {
    let logo = |a: &crate::agents::Agent| logos::agent(a.id).unwrap_or((&oc::CONSOLE, None));
    let default_label = s.agent().map_or("Default agent".to_string(), |a| format!("Default ({})", a.name));
    let label = chosen.map_or(default_label.clone(), |a| a.name.to_string());
    let mark = chosen.or(s.agent()).map_or((&oc::CONSOLE, None), logo);
    let mut menu = Menu::new(key.to_string())
        .trigger(menu_trigger(k, key, mark, label, toggle))
        .open(open)
        .min_width(260.0)
        .placement(rune_kit::Placement::BottomStart)
        .on_close(move |s: &mut App| close(s))
        .item(
            MenuItem::new(default_label)
                .lead(lead(k, s.agent().map_or((&oc::CONSOLE, None), logo)))
                .description("Changes with the default in Setup")
                .checked(chosen.is_none() || chosen.map(|a| a.id) == s.agent().map(|a| a.id))
                .on_select(move |s: &mut App| pick(s, None)),
        );
    // The default is already the first item.
    let (installed, missing): (Vec<_>, Vec<_>) =
        AGENTS.iter().filter(|a| Some(a.id) != s.agent().map(|d| d.id)).partition(|a| s.installed.contains(&a.id));
    if !installed.is_empty() {
        menu = menu.separator();
    }
    for a in installed {
        menu = menu.item(
            MenuItem::new(a.name)
                .lead(lead(k, logo(a)))
                .checked(chosen.is_some_and(|x| x.id == a.id))
                .on_select(move |s: &mut App| pick(s, Some(a))),
        );
    }
    if !missing.is_empty() {
        menu = menu.separator();
        for a in missing {
            menu = menu.item(MenuItem::new(a.name).lead(lead(k, logo(a))).description("Not installed").disabled(true));
        }
    }
    menu
}

/// The agent writing this report: the one it names, else the default.
fn agent_mark(s: &App, d: &Draft) -> logos::Mark {
    d.agent.or(s.agent()).and_then(|a| logos::agent(a.id)).unwrap_or((&oc::CONSOLE, None))
}

/// Where this report's result goes, as a logo: the place, the connected
/// tracker, the agent for "Your agent's tools", or the call's page.
fn destination_mark(s: &App, d: &Draft) -> logos::Mark {
    match d.to {
        Some(to) => match (to, s.connection(to)) {
            (_, Some(c)) => logos::for_place(c.place),
            (To::Place(p), None) => logos::for_place(p),
            (To::Issues, None) => (&oc::CHECKLIST, None),
        },
        None if d.sends_to.is_some() => agent_mark(s, d),
        None => (&oc::WAVEFORM, None),
    }
}

/// Run on each call as it finishes. Not for "Your agent's tools": the agent
/// would post with nobody there to approve it.
fn auto_switch(k: &Kit, d: &Draft) -> El {
    let agent_posts = d.sends_to.is_some();
    let about = if agent_posts {
        "Not with your agent's tools: nobody would approve what it posts."
    } else if d.to.is_some() {
        "Runs on each new call. Posting still waits for you."
    } else {
        "Runs on each new call; the result waits on the call."
    };
    col()
        .gap(4.0)
        .pt(4.0)
        .child(
            Switch::new(d.auto && !agent_posts)
                .label("When a call ends")
                .disabled(agent_posts)
                .on_toggle(|s: &mut App| {
                    if let Some(d) = s.editing.as_mut() {
                        d.auto = !d.auto;
                    }
                })
                .build(k)
                .key("report-auto"),
        )
        .child(text(about).font(k.ty.small).color(k.c.text_muted))
}

fn toggle(s: &mut App, menu: &'static str) {
    if let Some(d) = s.editing.as_mut() {
        d.menu = if d.menu == Some(menu) { None } else { Some(menu) };
    }
}

fn close(s: &mut App) {
    if let Some(d) = s.editing.as_mut() {
        d.menu = None;
    }
}

fn list(s: &App, k: &Kit, hair: f32) -> (El, El) {
    let mut list = col().key("report-list").w_full().gap(12.0);
    for e in &s.report_errors {
        list = list.child(super::notice(k, e, None));
    }
    if s.reports.is_empty() && s.report_errors.is_empty() {
        let new = Button::new("New report").variant(ButtonVariant::Submit).on_press(|s: &mut App| s.new_report()).build(k).key("new-report-empty");
        list = list.child(super::empty_state(
            k,
            "reports-empty",
            super::ghost_rows(k, tile(k, (&oc::WAVEFORM, None))),
            "No reports yet",
            "Every call already has a Summary. Make a report for anything else: customer feedback in Slack, a deal note in Notion, issues in GitHub.",
            vec![new],
        ));
    } else {
        list = list.child(group(k, "reports", hair, s.reports.iter().map(|r| report_row(s, k, r)).collect()));
    }
    let actions = vec![
        Button::new("Open folder")
            .variant(ButtonVariant::Neutral)
            .help("The reports are Markdown files you can also edit by hand")
            .on_press(|s: &mut App| {
                if !s.offline {
                    app::reveal(paths::reports_dir());
                }
            })
            .build(k)
            .key("open-reports"),
    ];
    // An empty page has its own New report, in the empty state.
    let actions = if s.reports.is_empty() {
        actions
    } else {
        actions.into_iter().chain([Button::new("New report").variant(ButtonVariant::Submit).on_press(|s: &mut App| s.new_report()).build(k).key("new-report")]).collect()
    };
    (header(k, "Reports", "What to make from a call, and where it goes.", actions), list)
}

pub fn page(s: &App, k: &Kit, hair: f32, layer: &mut Vec<El>) -> El {
    let (head, body) = match &s.editing {
        Some(d) => editor(s, k, d, hair, layer),
        None => list(s, k, hair),
    };
    super::page_shell("reports", head, body)
}
