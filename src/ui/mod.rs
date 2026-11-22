//! The window: a sidebar of pages beside the page itself, in the OpenCode
//! design language Rune's kit ships (the same tokens Hex's window uses).

use rune::{Cx, El, Icon, TextStyle, col, el, icon, row, spacer, text};
use rune_kit::Kit;
use rune_theme_opencode::icons as oc;

use crate::app::{App, HexStatus, Page};

mod about;
mod activity;
mod logos;
mod meetings;
mod reports;
mod setup;
mod start;

pub const SIDEBAR_W: f32 = 216.0;
pub const TITLE: TextStyle = TextStyle::new(17.0, 640.0).lh(24.0);
pub const HEADING: TextStyle = TextStyle::new(13.0, 600.0).lh(18.0);

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let dark = match s.settings.appearance.as_deref() {
        Some("light") => false,
        Some("dark") => true,
        _ => cx.system_dark(),
    };
    let k = rune_theme_opencode::kit(dark);
    let hair = 1.0 / cx.env().scale;
    // Menus (the language one) draw above everything, in this layer.
    let mut layer = Vec::new();
    let page = match s.page {
        Page::Meetings => meetings::page(s, k, hair, &mut layer),
        Page::Reports => reports::page(s, k, hair, &mut layer),
        Page::Activity => activity::page(s, k, hair),
        Page::Setup => setup::page(s, k, hair, &mut layer),
        Page::About => about::page(s, k),
    };
    let base = row()
        .key("window")
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(sidebar(s, k))
        .child(el().key("sidebar-edge").w(hair).h_full().fill(k.c.border_weak))
        .child(col().key("content").grow(1.0).shrink().min_w(0.0).h_full().min_h(0.0).child(page));
    rune_kit::root(base, layer)
}

fn nav(s: &App, k: &Kit, page: Page, label: &str, glyph: &'static Icon, count: Option<usize>) -> El {
    let on = s.page == page;
    let c = &k.c;
    let mut item = row()
        .key(format!("nav-{label}"))
        .role("tab")
        .label(label)
        .checked(on)
        .h(30.0)
        .px(8.0)
        .gap(8.0)
        .center_y()
        .radius(6.0)
        .fill(if on { c.layer_03 } else { rune::Color::TRANSPARENT })
        .hover(if on { rune::Color::TRANSPARENT } else { c.hover })
        .on_press(move |s: &mut App| s.go(page))
        .child(icon(glyph, 16.0).color(if on { c.icon_base } else { c.icon_muted }))
        .child(text(label).font(k.ty.ui).color(if on { c.text_base } else { c.text_muted }).grow(1.0));
    if let Some(n) = count.filter(|n| *n > 0) {
        item = item.child(text(n.to_string()).font(k.ty.small).color(c.text_faint));
    }
    item
}

fn sidebar(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let brand = row()
        .key("brand")
        .gap(10.0)
        .px(6.0)
        .pb(12.0)
        .center_y()
        .child(icon(&logos::APP, 32.0).a11y_hidden())
        .child(
            col()
                .gap(1.0)
                .child(text("Hex Meetings").role("heading").level(1).font(HEADING).color(c.text_strong))
                .child(text("Unofficial companion").font(k.ty.small).color(c.text_faint)),
        );
    let hex_line = match &s.hex {
        Some(HexStatus::Found { version, .. }) => {
            format!("Reading from Hex {}", version.as_deref().unwrap_or(""))
        }
        Some(HexStatus::Missing) => "Hex not found".into(),
        Some(HexStatus::Looking) | None => "Looking for Hex…".into(),
    };
    let dot = match &s.hex {
        Some(HexStatus::Found { .. }) => c.success.icon,
        Some(HexStatus::Missing) => c.danger.icon,
        _ => c.icon_faint,
    };
    col()
        .key("sidebar")
        .w(SIDEBAR_W)
        .h_full()
        .fill(c.page)
        .px(10.0)
        .pt(44.0)
        .pb(14.0)
        .gap(2.0)
        .window_drag()
        .child(brand)
        .child(nav(s, k, Page::Meetings, "Meetings", &oc::WAVEFORM, Some(s.meetings.len())))
        .child(nav(s, k, Page::Reports, "Reports", &oc::PROMPT, Some(s.reports.len())))
        .child(nav(s, k, Page::Activity, "Activity", &oc::STATUS, Some(s.activity.len())))
        .child(nav(s, k, Page::Setup, "Setup", &oc::SLIDERS, None))
        .child(nav(s, k, Page::About, "About", &oc::INFO, None))
        .child(spacer())
        .children(matches!(s.update, crate::app::Update::Available(_)).then(|| {
            rune_kit::Button::new("Update available")
                .icon(&oc::DOWNLOAD)
                .variant(rune_kit::ButtonVariant::Neutral)
                .control_size(rune_kit::ControlSize::Small)
                .on_press(|s: &mut App| s.go(crate::app::Page::About))
                .build(k)
                .key("update-available")
                .mx(8.0)
                .mb(10.0)
        }))
        .child(
            row()
                .key("hex-status")
                .gap(8.0)
                .px(8.0)
                .pb(8.0)
                .center_y()
                .child(el().size(7.0, 7.0).radius(3.5).fill(dot))
                .child(text(hex_line).font(k.ty.small).color(c.text_muted)),
        )
        .child(
            text("Not affiliated with Hex or Anomaly.")
                .key("disclaimer")
                .font(k.ty.text_xsmall)
                .color(c.text_faint)
                .wrap()
                .px(8.0),
        )
}

/// A button filled red, for actions that can't be undone. The kit's Danger
/// style only colours the label, so it's drawn here, at the kit's regular
/// button size. One deep red in both themes keeps white text readable.
pub fn destructive_button(k: &Kit, label: &str, key: &str, f: fn(&mut App)) -> El {
    let red = logos::srgb(0xC8323C);
    row()
        .key(key.to_string())
        .role("button")
        .label(label.to_string())
        .focusable(true)
        .h(28.0)
        .px(14.0)
        .center()
        .radius(k.radius.md)
        .fill(red)
        .hover(red.mix(rune::Color::BLACK, 0.15))
        .on_press(f)
        .child(text(label.to_string()).font(k.ty.ui_medium).color(rune::Color::WHITE))
}

/// A logo at `size`, in its brand colour, its own colours, or the theme's.
pub fn mark(k: &Kit, (glyph, own): logos::Mark, size: f32) -> El {
    icon(glyph, size).color(own.unwrap_or(k.c.icon_base))
}

/// An agent's logo at `size`.
pub fn agent_logo(k: &Kit, agent_id: &str, size: f32) -> Option<El> {
    logos::agent(agent_id).map(|m| mark(k, m, size))
}

/// A logo on a rounded tile: how every list row names an agent or a place.
pub fn tile(k: &Kit, logo: logos::Mark) -> El {
    el().size(TILE, TILE).radius(8.0).fill(k.c.bg_base).border(1.0, k.c.border_weak).center().child(mark(k, logo, 18.0))
}

const TILE: f32 = 32.0;
const ROW_PX: f32 = 14.0;
const ROW_GAP: f32 = 12.0;

/// One row of a [`group`]: a tile, a title with a line under it, then
/// whatever sits on the right (a button, a tag, a chevron).
pub fn list_row(k: &Kit, key: impl Into<String>, lead: El, title: &str, subtitle: Option<String>, trailing: Vec<El>) -> El {
    let mut words = col().grow(1.0).shrink().min_w(0.0).gap(2.0).child(text(title.to_string()).font(k.ty.ui_medium).color(k.c.text_base));
    if let Some(sub) = subtitle {
        words = words.child(text(sub).font(k.ty.small.lh(18.0)).color(k.c.text_muted).wrap());
    }
    row()
        .key(key.into())
        .w_full()
        .gap(ROW_GAP)
        .px(ROW_PX)
        .py(10.0)
        .center_y()
        .child(lead)
        .child(words)
        .children(trailing)
}

/// Rows in one raised container, split by hairlines that start where the
/// rows' text does, as a desktop settings list.
pub fn group(k: &Kit, key: &str, hair: f32, rows: Vec<El>) -> El {
    let mut out = col().key(key.to_string()).w_full().radius(k.radius.lg).fill(k.c.surface_raised_base).border(1.0, k.c.border_weak).clip_rounded(k.radius.lg);
    for (i, r) in rows.into_iter().enumerate() {
        if i > 0 {
            out = out.child(row().w_full().pl(ROW_PX + TILE + ROW_GAP).child(el().h(hair).grow(1.0).fill(k.c.border_weak)));
        }
        out = out.child(r);
    }
    out
}

/// A titled part of a page: a heading, an optional line under it and
/// actions on the right, then the content.
pub fn section(k: &Kit, key: &str, title: &str, about: Option<&str>, actions: Vec<El>, content: El) -> El {
    let mut head = col().grow(1.0).shrink().min_w(0.0).gap(2.0).child(text(title.to_string()).role("heading").level(2).font(HEADING).color(k.c.text_strong));
    if let Some(a) = about {
        head = head.child(text(a.to_string()).font(k.ty.small).color(k.c.text_muted).wrap());
    }
    col().key(key.to_string()).w_full().gap(10.0).child(row().w_full().px(2.0).gap(12.0).items_end().child(head).children(actions)).child(content)
}

/// The menu for the language calls are in.
pub fn language_select(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let names: Vec<&str> = crate::speech::LANGUAGES.iter().map(|(_, n)| *n).collect();
    rune_kit::Select::new("language", &names)
        .label("Language of your calls")
        .current(Some(crate::speech::name(s.language())))
        .open(s.language_open)
        .width(170.0)
        .on_toggle(|s: &mut App| s.language_open = !s.language_open)
        .on_close(|s: &mut App| s.language_open = false)
        .on_choose(|s: &mut App, name: &str, _| {
            if let Some(code) = crate::speech::code(name) {
                s.set_language(code);
            }
        })
        .build(k, layer)
}

/// Whether Hex's speech model transcribes the calls' language, and the
/// sentence that says so (or names the models that would).
pub fn speech_status(s: &App) -> (bool, String) {
    use crate::speech::{Fit, fit, mismatch_sentence, name};
    let lang = s.language();
    match &s.overview {
        None => (false, "Reading Hex's speech models…".into()),
        Some(Err(_)) => (false, "Open Hex to check its speech model.".into()),
        Some(Ok(o)) => match fit(&o.models, lang) {
            Fit::Fits(m) => (true, format!("{} transcribes {}.", m.name, name(lang))),
            Fit::Mismatch { current, better } => (false, mismatch_sentence(current, &better, lang)),
            Fit::Unknown => (false, "Hex hasn't said which speech model it uses.".into()),
        },
    }
}

/// What an empty list shows: a picture of the list to come, a title, one
/// line on what will appear here, and the way to get there.
pub fn empty_state(k: &Kit, key: &str, art: El, title: &str, body: &str, actions: Vec<El>) -> El {
    col()
        .key(key.to_string())
        .w_full()
        .items_center()
        .gap(10.0)
        .px(24.0)
        .py(36.0)
        .child(art.mb(10.0))
        .child(text(title.to_string()).font(HEADING).color(k.c.text_strong))
        .child(text(body.to_string()).font(k.ty.ui).color(k.c.text_muted).wrap().max_w(420.0).align(rune::Align::Center))
        .child(row().gap(8.0).mt(8.0).children(actions))
}

/// The picture for an empty list: the row that will appear, crisp, between
/// two faded placeholders, so it's clear something fills this space.
pub fn ghost_rows(k: &Kit, lead: El) -> El {
    let c = &k.c;
    let bar = |w: f32, h: f32, fill: rune::Color| el().w(w).h(h).radius(h / 2.0).fill(fill);
    let ghost = |w: f32| {
        row()
            .w(w)
            .h(40.0)
            .px(12.0)
            .gap(10.0)
            .center_y()
            .radius(10.0)
            .fill(c.bg_base)
            .border(1.0, c.border_weak)
            .opacity(0.45)
            .child(el().size(20.0, 20.0).radius(6.0).fill(c.layer_02))
            .child(col().gap(5.0).child(bar(w * 0.42, 6.0, c.layer_03)).child(bar(w * 0.26, 5.0, c.layer_02)))
    };
    let main = row()
        .w(280.0)
        .h(52.0)
        .px(12.0)
        .gap(12.0)
        .center_y()
        .radius(12.0)
        .fill(c.bg_base)
        .border(1.0, c.border_base)
        .child(lead)
        .child(col().gap(6.0).child(bar(120.0, 7.0, c.text_faint)).child(bar(80.0, 6.0, c.layer_03)));
    col().items_center().gap(8.0).child(ghost(236.0)).child(main).child(ghost(236.0))
}

/// The empty state for a narrow column: an icon, a line, and maybe a button.
pub fn empty_small(k: &Kit, key: &str, glyph: &'static Icon, line: &str, action: Option<El>) -> El {
    col()
        .key(key.to_string())
        .w_full()
        .items_center()
        .gap(10.0)
        .py(32.0)
        .px(12.0)
        .child(el().size(36.0, 36.0).radius(10.0).fill(k.c.layer_02).center().child(icon(glyph, 18.0).color(k.c.icon_muted)))
        .child(text(line.to_string()).font(k.ty.ui).color(k.c.text_muted).wrap().align(rune::Align::Center))
        .children(action)
}

/// A page: its header pinned at the top, the rest scrolling under it. Pinned,
/// the header's buttons never slide under the scroll area's edge.
pub fn page_shell(key: &str, header: El, body: El) -> El {
    col()
        .key(format!("{key}-page"))
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(row().w_full().h(40.0).window_drag())
        // The column is centred, so a wide window doesn't leave it stranded on the left.
        .child(col().w_full().px(32.0).pb(16.0).items_center().child(col().w_full().max_w(720.0).child(header)))
        .child(
            col()
                .key(format!("{key}-scroll"))
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .px(32.0)
                .pt(4.0)
                .pb(32.0)
                .items_center()
                .child(col().w_full().max_w(720.0).child(body))
                .scroll(rune::ScrollSpec { stick_bottom: false }),
        )
}

/// A page's title and one line under it, with actions on the right.
pub fn header(k: &Kit, title: &str, description: &str, actions: Vec<El>) -> El {
    header_with(k, title, None, description, actions)
}

/// [`header`] with something before the line under the title, such as a logo.
pub fn header_with(k: &Kit, title: &str, lead: Option<El>, description: &str, actions: Vec<El>) -> El {
    row()
        .key("page-header")
        .w_full()
        .gap(12.0)
        .items_start()
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(4.0)
                .child(text(title).role("heading").level(1).font(TITLE).color(k.c.text_strong))
                .child(
                    row()
                        .gap(6.0)
                        .center_y()
                        .children(lead)
                        .child(text(description).font(k.ty.ui).color(k.c.text_muted).wrap().shrink().min_w(0.0)),
                ),
        )
        .children(actions)
}

/// A small coloured label: Done, Failed, Recording.
pub fn tag(k: &Kit, label: &str, tone: Tone) -> El {
    let (bg, fg) = match tone {
        Tone::Good => (k.c.success.bg, k.c.success.fg),
        Tone::Bad => (k.c.danger.bg, k.c.danger.fg),
        Tone::Busy => (k.c.info.bg, k.c.info.fg),
        Tone::Quiet => (k.c.layer_02, k.c.text_muted),
    };
    text(label).font(k.ty.text_xsmall_medium).color(fg).fill(bg).px(6.0).py(1.0).radius(4.0)
}

#[derive(Clone, Copy)]
pub enum Tone {
    Good,
    Bad,
    Busy,
    Quiet,
}

pub fn outcome_tone(o: crate::activity::Outcome) -> Tone {
    match o {
        crate::activity::Outcome::Done => Tone::Good,
        crate::activity::Outcome::Failed => Tone::Bad,
        crate::activity::Outcome::Stopped => Tone::Quiet,
    }
}

/// A notice above a page's content, with a way to dismiss it.
pub fn notice(k: &Kit, message: &str, dismiss: Option<fn(&mut App)>) -> El {
    let mut row = row()
        .key("notice")
        .w_full()
        .gap(8.0)
        .px(12.0)
        .py(8.0)
        .radius(k.radius.md)
        .fill(k.c.warning.bg)
        .center_y()
        .child(icon(&oc::WARNING, 14.0).color(k.c.warning.icon))
        .child(text(message).font(k.ty.ui).color(k.c.warning.fg).wrap().grow(1.0).shrink().min_w(0.0));
    if let Some(f) = dismiss {
        row = row.child(
            rune_kit::IconButton::new(&oc::CLOSE_SMALL)
                .variant(rune_kit::IconButtonVariant::Ghost)
                .label("Dismiss")
                .on_press(f)
                .build(k),
        );
    }
    row
}
