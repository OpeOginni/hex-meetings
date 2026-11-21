//! The window: a sidebar of pages beside the page itself, in the OpenCode
//! design language Rune's kit ships (the same tokens Hex's window uses).

use rune::{Cx, El, Icon, TextStyle, col, el, icon, row, spacer, text};
use rune_kit::Kit;
use rune_theme_opencode::icons as oc;

use crate::app::{App, HexStatus, Page};

mod activity;
mod meetings;
mod recipes;

pub const SIDEBAR_W: f32 = 216.0;
pub const TITLE: TextStyle = TextStyle::new(17.0, 640.0).lh(24.0);
pub const HEADING: TextStyle = TextStyle::new(13.0, 600.0).lh(18.0);

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let k = rune_theme_opencode::kit(cx.system_dark());
    let hair = 1.0 / cx.env().scale;
    let page = match s.page {
        Page::Meetings => meetings::page(s, k, hair),
        Page::Recipes => recipes::page(s, k),
        Page::Activity => activity::page(s, k),
    };
    let base = row()
        .key("window")
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(sidebar(s, k))
        .child(el().key("sidebar-edge").w(hair).h_full().fill(k.c.border_weak))
        .child(col().key("content").grow(1.0).shrink().min_w(0.0).h_full().min_h(0.0).child(page));
    rune_kit::root(base, Vec::new())
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
        .on_press(move |s: &mut App| s.page = page)
        .child(icon(glyph, 16.0).color(if on { c.icon_base } else { c.icon_muted }))
        .child(text(label).font(k.ty.ui).color(if on { c.text_base } else { c.text_muted }).grow(1.0));
    if let Some(n) = count.filter(|n| *n > 0) {
        item = item.child(text(n.to_string()).font(k.ty.small).color(c.text_faint));
    }
    item
}

fn sidebar(s: &App, k: &Kit) -> El {
    let c = &k.c;
    let brand = col()
        .key("brand")
        .gap(2.0)
        .px(8.0)
        .pb(12.0)
        .child(text("Hex Meetings").role("heading").level(1).font(HEADING).color(c.text_strong))
        .child(text("Unofficial companion").font(k.ty.small).color(c.text_faint));
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
        .child(nav(s, k, Page::Recipes, "Recipes", &oc::PROMPT, Some(s.recipes.len())))
        .child(nav(s, k, Page::Activity, "Activity", &oc::STATUS, Some(s.activity.len())))
        .child(spacer())
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
            text("Not affiliated with Hex or Anomaly. Reads Hex's public command line.")
                .key("disclaimer")
                .font(k.ty.text_xsmall)
                .color(c.text_faint)
                .wrap()
                .px(8.0),
        )
}

/// A page's title and one line under it, with actions on the right.
pub fn header(k: &Kit, title: &str, description: &str, actions: Vec<El>) -> El {
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
                .child(text(description).font(k.ty.ui).color(k.c.text_muted).wrap()),
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
