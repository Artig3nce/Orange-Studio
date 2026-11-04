// Copyright (C) 2026 ItsAlphaNeon
// Copyright (C) 2026 Artig3nce
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Views of the main window, laid out from the measured default layout.
// spec: ui/main-window.md

pub mod audio_form;
pub mod browser;
pub mod plugin_panel;
pub mod plugin_picker;
pub mod cabledebug;
pub mod channel_win;
pub mod data_folder;
pub mod factory_folder;
pub mod dialog;
pub mod menus;
pub mod midi_form;
pub mod midi_page;
pub mod mondebug;
pub mod palette;
pub mod rolling;
pub mod settings;
pub mod mixer;
pub mod piano;
pub mod patterns;
pub mod perf;
pub mod playlist;
pub mod project_forms;
pub mod rack;
pub mod remote_box;
pub mod export_form;
pub mod save_form;
pub mod arpeggiate;
pub mod strum;
pub mod piano_dlg;
pub mod tapper;
pub mod touch;
pub mod events;
pub mod theme_preview;
pub mod toolbar;
pub mod typein;

use std::borrow::Cow;

use os_theme::Palette;
use os_ui::{kit, Color, Rect};

use crate::app::{Action, Bar, Scroll, Seg, State, Ui, Win};
use crate::icons::{self, Icon};

pub(super) const MANGO_MIXER_RIM: Color = Color::rgb(0x2e2e2e);

/// The theme's `display_edge` hairline around a display or window readout, inset half a pixel so it stays one
/// crisp line at any scale. Shared by the toolbar readouts and every window rim.
pub fn display_edge(c: &mut os_ui::Canvas, t: &os_theme::Palette, r: Rect, radius: f32) {
    c.stroke_rrect(r.inset(0.5), radius, 1.0, t.display_edge);
}

/// Per-frame values that don't live in `State`.
pub struct FrameInfo {
    pub title: String,
    pub hint: Cow<'static, str>,
    pub playing: bool,
    /// Playing but held by Play (pause).
    pub paused: bool,
    /// Playback position in ticks of the running mode (a pattern wraps at its end), and whether that mode is
    /// the playlist. Every playhead and the time display read these, never the UI's own tempo or mode.
    pub tick: f64,
    pub song: bool,
    /// The tempo the toolbar shows: the one playback plays at (tempo automation moves it), else the project's.
    pub tempo: f64,
    pub peaks: (f32, f32),
    /// Notes playing now, as (channel index, key).
    pub sounding: Vec<(u16, u16)>,
    /// The song position slider's handle is being dragged (it shows the dragged position, not the playhead).
    pub song_pos_drag: bool,
    /// The mixer track whose fader is being dragged (the Orange skin lights its bar).
    pub fader_drag: Option<u16>,
    /// What the toolbar editor's drag carries (spec: ui/toolbar-menu.md §8).
    pub bar_drag: Option<crate::toolbar::BarDrag>,
}

/// UI text at a nominal size (Cuprum; spec §10).
pub fn fnt(size: f32) -> os_ui::Font {
    os_ui::Font::new(size.round() as u16)
}

/// The light display face (song position).
pub fn light(size: f32) -> os_ui::Font {
    os_ui::Font::light(size.round() as u16, 0)
}

/// Display scale of popup menus and combo lists for a window at scale `main`: General ▸ Display ▸ Pop-ups scaling
/// (Main = `main`). Their row heights follow it (`os_ui::kit::menu::metrics`). Spec: ui/ui-scaling.md §1–2.
pub fn popup_scale(main: f32) -> f32 {
    let pct = crate::prefs::choice(&crate::prefs::POPUPS_SCALING).strip_suffix('%').and_then(|p| p.parse::<f32>().ok());
    pct.map_or(main, |p| p / 100.0)
}

/// Top of the workspace (§1) below the toolbar of two rows (the classic layout).
pub const WORK_Y: f32 = 83.0;

/// Top of the workspace and the browser now: the window's top while the toolbar is hidden (spec: ui/toolbar-menu.md
/// §5), else the top band's bottom (`band_h`) — plus the rolling sampler's strip when it shows (issue #82): the
/// strip is docked between the toolbar and the playlist, and everything below moves with it.
pub fn work_y(s: &State) -> f32 {
    let band = if s.toolbar.hidden { 0.0 } else { band_h(s) };
    band + rolling::strip_h(s)
}

/// Display scale of the main window's top band (the toolbar rows, `WORK_Y` tall in its own px for two rows) for a
/// window at scale `main`: General ▸ Display ▸ Toolbars scaling (Main = `main`). Spec: ui/ui-scaling.md §1.
pub fn toolbar_scale(main: f32) -> f32 {
    let pct = crate::prefs::choice(&crate::prefs::TOOLBARS_SCALING).strip_suffix('%').and_then(|p| p.parse::<f32>().ok());
    pct.map_or(main, |p| p / 100.0)
}

/// The least a band that auto fits paints at, of its own px per window px (`band_k`): below it the far panels are left
/// out again (`toolbar::room`) and widening the window or lowering Toolbars scaling is the only way.
const BAND_FIT_MIN: f32 = 0.5;

/// The band's scale for a window `width` logical px wide (its rows, layout `l`, `editing`): Toolbars scaling, lowered
/// when the band at it is wider than the window, so every panel keeps showing instead of being left out from the right
/// — the output monitor's waveform display among them, gone on the small windows of Wine and Winlator (2026-10-27).
/// A scaling chosen by name ("Main", the default) only: a percentage is the user's measured wish and is kept. While
/// the toolbar editor is up the scale is kept too (its palette needs the band's full size to drag on). At
/// `BAND_FIT_MIN` it stops: narrower than that, the ▶ arrow comes back (`toolbar::room`).
pub fn toolbar_scale_fit(main: f32, width: f32, l: &crate::toolbar::Layout, editing: bool) -> f32 {
    let k = toolbar_scale(main) / main.max(0.01);
    if editing || !crate::prefs::is_main_choice(&crate::prefs::TOOLBARS_SCALING) {
        return k * main.max(0.01);
    }
    // Window px per band px at which every panel shows: the band's full natural width plus the ▶ arrow's room.
    let full = crate::toolbar::natural_width(l, editing) + toolbar::MORE_W;
    let k = k.min((width / full).max(BAND_FIT_MIN));
    k * main.max(0.01)
}

/// The window's px per px of the top band at the last paint's scale: 1 at Toolbars scaling Main.
pub fn band_k(s: &State) -> f32 {
    let main = s.ui_scale.max(0.01);
    toolbar_scale_fit(main, s.screen.0, &s.bar.layout, s.bar.editing) / main
}

/// Height of the top band in the window's px (shown or not): its rows' (`toolbar::Layout::band_h`, `WORK_Y` for two)
/// at its scale.
pub fn band_h(s: &State) -> f32 {
    s.bar.layout.band_h() * band_k(s)
}

/// `r` in px `k` times as big.
pub fn scale_rect(r: Rect, k: f32) -> Rect {
    Rect::new(r.x * k, r.y * k, r.w * k, r.h * k)
}

/// The top band's toolbar panels as laid out now, in the band's own px.
fn placed(s: &State) -> Vec<crate::toolbar::Placed> {
    crate::toolbar::place(&s.bar.layout, s.screen.0 / band_k(s), s.bar.editing)
}

/// How far toolbar panel `p` is from its home (`toolbar::Panel::home`); `None` when it is not on the toolbar.
pub fn panel_at(s: &State, p: crate::toolbar::Panel) -> Option<crate::toolbar::At> {
    placed(s).into_iter().find(|q| q.panel == p).map(|q| q.at())
}

/// Rect `r` of the top band, given in the home px of the panel whose slot holds its top-left (`toolbar::Panel`,
/// `app::HINT_RECT`, `patterns::PLUS_BOX_AT`...), in the window's px where that panel is drawn now. A panel that is not
/// on the toolbar leaves `r` where it is (scaled).
pub fn band_rect(s: &State, r: Rect) -> Rect {
    let a = crate::toolbar::Panel::at_home(r.x, r.y).and_then(|p| panel_at(s, p)).unwrap_or_default();
    scale_rect(a.r(r), band_k(s))
}

/// The parts of rect `r` of the top band, given in its panels' home px (`band_rect`), in the window's px where each of
/// those panels is drawn now: to repaint them (`App::damage_band`). Panels that are not on the toolbar give none.
pub fn band_rects(s: &State, r: Rect) -> Vec<Rect> {
    let k = band_k(s);
    placed(s)
        .into_iter()
        .filter(|p| !p.panel.repeats())
        .filter_map(|p| {
            let (row, x0, x1) = p.panel.home();
            let slot = Rect::new(x0, crate::toolbar::ROW_H * row as f32, x1 - x0, crate::toolbar::ROW_H + 1.0);
            let part = slot.intersect(&r);
            (!part.is_empty()).then(|| scale_rect(p.at().r(part), k))
        })
        .collect()
}

/// The toolbar editor's palette in the window's px, while it is open (spec: ui/toolbar-menu.md §8).
pub fn palette_rect(s: &State) -> Option<Rect> {
    let k = band_k(s);
    (s.bar.editing && !s.toolbar.hidden).then(|| scale_rect(toolbar::palette(s, s.screen.0 / k).rect, k))
}
/// Width of the docked browser incl. the splitter (§4): the default, and the least its splitter drags it to.
pub const BROWSER_W: f32 = 209.0;

/// Width of the docked browser incl. the splitter, as dragged (`State::browser_w`), at most half the window (spec:
/// ui/browser.md §9).
pub fn browser_w(s: &State) -> f32 {
    s.browser_w.min(s.screen.0 / 2.0).max(BROWSER_W)
}

/// The theme's colours for the docked layout: the bars, panels and the Playlist's grid all on one background (the
/// theme's window colour), so they read as one surface.
pub fn docked_palette(p: &Palette) -> Palette {
    let c = p.window_title;
    let mut d = p.clone();
    for f in [
        &mut d.toolbar,
        &mut d.toolbar_line,
        &mut d.menu_panel,
        &mut d.workspace,
        &mut d.display_edge,
        &mut d.browser_title,
        &mut d.browser_tabs,
        &mut d.browser_list,
        &mut d.browser_footer,
        &mut d.splitter,
        &mut d.splitter_dark,
        &mut d.window_title_float,
        &mut d.window_title_hi,
        &mut d.window_body,
        &mut d.picker,
        &mut d.track_header,
        &mut d.scrollbar,
        &mut d.ruler,
        &mut d.grid,
        &mut d.grid_gap,
        &mut d.mixer_bg,
        &mut d.mixer_panel,
        &mut d.rack_inset,
    ] {
        *f = c;
    }
    // Every other group of bars a little lighter, to count them by.
    d.grid_alt = c.shade(0.035);
    // The piano roll on the same ground, its black keys' rows a little darker.
    d.pr_row = c;
    d.pr_row_black = c.mix(Color::rgb(0x000000), 0.2);
    // The piano's key letters, closer to black.
    d.key_label = p.key_label.mix(Color::rgb(0x000000), 0.45);
    d
}

/// The docked layout's bottom panel: a tab for each of its windows, and past them the strip that resizes the panel.
fn panel_tabs(ui: &mut Ui, s: &State) {
    let a = s.panel_area();
    let strip = Rect::new(a.x, a.y, a.w, crate::app::PANEL_TABS_H);
    let t = ui.t;
    ui.c.fill_rect(strip, t.window_title);
    ui.c.fill_rect(Rect::new(strip.x, strip.y, strip.w, 1.0), t.splitter_dark);
    ui.hit(strip, Action::PanelSplitter, "Drag to resize the panel");
    let font = fnt(16.0);
    let mut x = strip.x + 8.0;
    for (win, name) in [(Win::Mixer, "Mixer"), (Win::PianoRoll, "Piano roll"), (Win::ChannelRack, "Channel rack")] {
        let w = ui.c.text_width(font, name) + 28.0;
        let r = Rect::new(x, strip.y, w, strip.h);
        let hov = ui.hit(r, Action::PanelTab(win), name);
        let open = s.panel == win;
        ui.c.text(font, x + 14.0, strip.y + 6.0, if open || hov { t.text } else { t.text_dim }, name);
        if open {
            ui.c.fill_rect(Rect::new(x + 6.0, strip.bottom() - 2.0, w - 12.0, 2.0), t.accent);
        }
        x += w;
    }
    // The device chain's tab: the plugins of the selected track as cards, side by side.
    {
        let w = ui.c.text_width(font, "Devices") + 28.0;
        let r = Rect::new(x, strip.y, w, strip.h);
        let hov = ui.hit(r, Action::PanelTab(Win::PluginTab), "Devices: the selected track's plugins");
        let open = s.panel == Win::PluginTab;
        ui.c.text(font, x + 14.0, strip.y + 6.0, if open || hov { t.text } else { t.text_dim }, "Devices");
        if open {
            ui.c.fill_rect(Rect::new(x + 6.0, strip.bottom() - 2.0, w - 12.0, 2.0), t.accent);
        }
    }
    // The chain scrolls when it is wider than the panel.
    if s.panel == Win::PluginTab && s.device_total > a.w + 1.0 && !s.panel_collapsed {
        for (dx, dir, hint) in [(-96.0, -1i8, "Scroll the devices left"), (-66.0, 1i8, "Scroll the devices right")] {
            let r = Rect::new(strip.cx() + dx, strip.y + 5.0, 26.0, 20.0);
            let hov = ui.hit(r, Action::DeviceScroll(dir), hint);
            ui.c.fill_rrect(r, 10.0, if hov { t.button_hi } else { t.sunken });
            let (cx, cy, tip) = (r.cx(), r.cy(), 3.0 * f32::from(-dir));
            ui.c.polyline(&[(cx + tip, cy - 5.0), (cx - tip, cy), (cx + tip, cy + 5.0)], 2.0, if hov { t.text } else { t.text_dim });
        }
    }
    // In the middle: the button that folds the panel away, so the Playlist has the whole height, and brings it back.
    let r = Rect::new(strip.cx() - 26.0, strip.y + 5.0, 52.0, 20.0);
    let hov = ui.hit(r, Action::PanelCollapse, if s.panel_collapsed { "Show the panel" } else { "Collapse the panel: more room for the playlist" });
    ui.c.fill_rrect(r, 10.0, if hov { t.button_hi } else { t.sunken });
    let (cx, cy) = (r.cx(), r.cy());
    // A chevron: down to fold the panel away, up to bring it back.
    let tip = if s.panel_collapsed { -3.0 } else { 3.0 };
    ui.c.polyline(&[(cx - 6.0, cy - tip), (cx, cy + tip), (cx + 6.0, cy - tip)], 2.0, if hov { t.text } else { t.text_dim });
}

/// The Devices tab's own parts: a thin strip for each folded card (a click opens it), and what to do when the track has no
/// plugin. The open cards are native windows over the panel.
fn device_chain(ui: &mut Ui, s: &State) {
    if s.panel != Win::PluginTab || s.panel_collapsed {
        return;
    }
    let a = s.panel_area();
    let panel = Rect::new(a.x, a.y + crate::app::PANEL_TABS_H, a.w, a.h - crate::app::PANEL_TABS_H);
    let t = ui.t;
    ui.c.push_clip(panel);
    // The panel's surface, over the workspace's watermark.
    ui.c.fill_rect(panel, os_ui::kit::lemon::plug::BG);
    if s.device_cards.is_empty() {
        let lines = ["No devices on this track", "Add an effect to one of its mixer slots, or drag one from the browser"];
        ui.c.text_center(fnt(16.0), Rect::new(panel.x, panel.cy() - 22.0, panel.w, 22.0), t.text_dim, lines[0]);
        ui.c.text_center(fnt(13.0), Rect::new(panel.x, panel.cy(), panel.w, 18.0), t.text_faint, lines[1]);
    }
    // An open card's caption instead of a title bar on the plugin (the plugin's own design fills the card): its name, a
    // Presets box with previous / next arrows (the preset it holds, or "Presets"), and on the mouse's card the close that
    // folds it up.
    for c in s.device_cards.iter().filter(|c| !c.folded) {
        let r = Rect::new(c.x, panel.y + 3.0, c.w, crate::app::DEVICE_CAPTION_H - 4.0);
        let over = ui.hovering(Rect::new(c.x, panel.y, c.w, panel.h));
        let font = fnt(13.0);
        let box_w = if c.w >= 250.0 { 158.0 } else { 0.0 };
        let close_w = if over { 24.0 } else { 0.0 };
        let name_w = (c.w - 16.0 - box_w - close_w).max(0.0);
        ui.c.text_clip(font, r.x + 8.0, r.y + 1.0, name_w, if over { t.text } else { t.text_dim }, &c.name);
        let mut right = r.right() - 4.0;
        if over {
            let close = Rect::new(right - 20.0, r.y, 20.0, r.h);
            let cov = ui.hit(close, Action::DeviceClose(c.row), format!("{}: fold up", c.name));
            let ink = if cov { t.text } else { t.text_dim };
            ui.c.line(close.cx() - 4.0, close.cy() - 4.0, close.cx() + 4.0, close.cy() + 4.0, 1.5, ink);
            ui.c.line(close.cx() + 4.0, close.cy() - 4.0, close.cx() - 4.0, close.cy() + 4.0, 1.5, ink);
            right = close.x - 4.0;
        }
        if box_w > 0.0 {
            // ◂ [ preset name ▾ ] ▸ : one rounded box, the arrows at its ends, the name opening the list.
            let bx = Rect::new(right - box_w, r.y, box_w, r.h);
            ui.c.fill_rrect(bx, bx.h / 2.0, Color::rgb(0x1f1f1f));
            let (prev, next) = (Rect::new(bx.x, bx.y, 22.0, bx.h), Rect::new(bx.right() - 22.0, bx.y, 22.0, bx.h));
            let (name_r, hp, hn) = (Rect::span(prev.right(), next.x, bx.y, bx.bottom()), ui.hit(prev, Action::DevicePresetStep(c.row, -1), "Previous preset"), ui.hit(next, Action::DevicePresetStep(c.row, 1), "Next preset"));
            let hl = ui.hit(name_r, Action::DevicePresets(c.row), "Presets");
            if hl {
                ui.c.fill_rrect(name_r.inset(1.0), 6.0, Color::rgb(0x2a2a2a));
            }
            let arrow = |ui: &mut Ui, r: Rect, left: bool, hov: bool| {
                let ink = if hov { t.text } else { t.text_dim };
                let (cx, cy, d) = (r.cx(), r.cy(), if left { -1.0 } else { 1.0 });
                ui.c.fill_polygon(&[(cx + 3.0 * d, cy - 4.0), (cx + 3.0 * d, cy + 4.0), (cx - 3.0 * d, cy)], ink);
            };
            arrow(ui, prev, true, hp);
            arrow(ui, next, false, hn);
            let label = c.preset.as_deref().unwrap_or("Presets");
            ui.c.text_clip(fnt(12.0), name_r.x + 6.0, name_r.y + 2.0, name_r.w - 18.0, if c.preset.is_some() || hl { t.text } else { t.text_dim }, label);
            let (cx, cy) = (name_r.right() - 8.0, name_r.cy());
            ui.c.fill_polygon(&[(cx - 3.5, cy - 1.5), (cx + 3.5, cy - 1.5), (cx, cy + 2.5)], t.text_dim);
        }
    }
    for c in s.device_cards.iter().filter(|c| c.folded) {
        let r = Rect::new(c.x, panel.y + 4.0, c.w, panel.h - 8.0);
        let hov = ui.hit(r, Action::DeviceFold(c.row), format!("{}: click to open", c.name));
        os_ui::kit::lemon::plug_panel(ui.c, r);
        let ink = if hov { t.text } else { t.text_dim };
        // A caret, then the name stacked down the strip.
        let (cx, y0) = (r.cx(), r.y + 12.0);
        ui.c.fill_polygon(&[(cx - 3.0, y0 - 4.0), (cx + 3.0, y0), (cx - 3.0, y0 + 4.0)], ink);
        let mut y = y0 + 12.0;
        for ch in c.name.chars().filter(|ch| !ch.is_whitespace()) {
            if y + 12.0 > r.bottom() - 6.0 {
                break;
            }
            let mut b = [0; 4];
            ui.c.text_center(fnt(12.0), Rect::new(r.x, y, r.w, 13.0), ink, ch.encode_utf8(&mut b));
            y += 12.0;
        }
    }
    ui.c.pop_clip();
}

/// Paints the frame; returns the number of hits registered before the overlays (menus, popups, picker...), which cover
/// docked plugin windows.
pub fn paint(ui: &mut Ui, s: &State, f: &FrameInfo) -> usize {
    let (w, h) = ui.c.size();
    // A maximized Playlist or Piano roll paints all of its rect (`framed`): the workspace under it needs no clear.
    let cover = s.docked().filter(|w| matches!(w, Win::Playlist | Win::PianoRoll)).map(|w| s.win_rect(w));
    ui.c.set_cover(cover);
    ui.c.clear(ui.t.workspace);
    ui.c.set_cover(None);

    // Workspace (§9): the theme's picture or the watermark, then docked windows on top.
    let left = if s.show_browser { browser_w(s) } else { 0.0 };
    let top = work_y(s);
    let work = Rect::new(left, top, w - left, h - top);
    let docked = s.docked();
    // spec: ui/flstheme.md §3 (BackMode, BackPicFilename); no watermark over a picture (`wallpaper`).
    if docked.is_none() && !crate::wallpaper::paint(ui, work) {
        let wm = ui.t.watermark;
        icons::wordmark(ui.c, work.cx(), work.cy() + 40.0, 560.0, ui.t.workspace, wm);
        let version = concat!("Orange Studio v", env!("CARGO_PKG_VERSION"));
        let vc = ui.t.version_text;
        let vw = ui.c.text_width(crate::views::fnt(16.0), version);
        ui.c.text(crate::views::fnt(16.0), work.cx() - vw / 2.0, h - 22.0, vc, version);
    }

    // One stacking order for all child windows (spec: ui/channel-rack.md §5). A maximized window
    // fills the workspace, so windows below it are covered and not painted.
    let order = s.paint_order();
    let docked_at = order.iter().rposition(|&w| s.is_max(w));
    // Each part is skipped by a repaint that misses it (`Ui::part`).
    if s.show_browser {
        let r = Rect::new(0.0, top, browser_w(s), browser::bottom(s) - top);
        ui.part(Seg::Browser, r, |ui| browser::paint(ui, s, r));
    }
    let _ = work;
    piano::set_compact(s.docked_ui);
    if s.docked_ui {
        panel_tabs(ui, s);
        device_chain(ui, s);
    }
    for (i, &win) in order.iter().enumerate() {
        if docked_at.is_some_and(|d| i < d) || (s.docked_ui && s.panel_collapsed && win.is_panel()) {
            continue;
        }
        ui.win = Some(win);
        let r = s.win_rect(win);
        // A window paints inside its paint area (also what it marks as drawn, e.g. text's generous overhang).
        let a = paint_area(s, win);
        ui.c.push_clip(a);
        let first_hit = ui.hits_len();
        match win {
            Win::Playlist => ui.part(Seg::Win(win), a, |ui| playlist::paint(ui, s, r, f)),
            Win::Mixer => ui.part(Seg::Win(win), a, |ui| {
                // Timed while the cables swing: a mixer too slow to redraw them hangs them still. Not in tests: they
                // paint while the rest of the suite runs, and on a busy machine the clock would hang the cables (and
                // post their message) at random between two paints a test compares. `cable`'s tests give the costs.
                let swinging = (!cfg!(test) && s.cables.borrow().moving()).then(std::time::Instant::now);
                mixer::paint(ui, s, r, f);
                if let Some(t) = swinging {
                    s.cables.borrow_mut().paint_cost(t.elapsed());
                }
            }),
            Win::PianoRoll => ui.part(Seg::Win(win), a, |ui| piano::paint(ui, s, r, f)),
            Win::ChannelRack => ui.part(Seg::Win(win), a, |ui| rack::paint(ui, s, f)),
            Win::Plugin => ui.part(Seg::Win(win), a, |ui| channel_win::paint(ui, s, f)),
            Win::Perf => ui.part(Seg::Win(win), a, |ui| perf::paint(ui, s, f)),
            Win::Tapper => ui.part(Seg::Win(win), a, |ui| tapper::paint(ui, s, f)),
            Win::Touch => ui.part(Seg::Win(win), a, |ui| touch::paint(ui, s, f)),
            Win::Strum => ui.part(Seg::Win(win), a, |ui| strum::paint(ui, s, f)),
            Win::Arp => ui.part(Seg::Win(win), a, |ui| arpeggiate::paint(ui, s, f)),
            Win::PianoDlg => ui.part(Seg::Win(win), a, |ui| piano_dlg::paint(ui, s, f)),
            Win::Events => ui.part(Seg::Win(win), a, |ui| events::paint(ui, s, f)),
            _ => {}
        }
        // The docked windows touch: a Playlist row running on past the window's edge must not take the panel's clicks.
        if s.is_fixed(win) {
            ui.keep_hits_in(first_hit, a);
        }
        ui.c.pop_clip();
        ui.win = None;
    }
    if !s.toolbar.hidden {
        // At Toolbars scaling (spec: ui/ui-scaling.md §1): the band works in its own px, `k` of the window's each, so
        // it is `w / k` of them wide; what doesn't fit is left out (`toolbar::room`). On a window too narrow for every
        // panel, the scale lowers instead (`toolbar_scale_fit`), so none is left out.
        let scale = toolbar_scale_fit(ui.c.scale, w, &s.bar.layout, s.bar.editing);
        ui.part(Seg::Toolbar, Rect::new(0.0, 0.0, w, band_h(s)), |ui| ui.scaled(scale, |ui, k| toolbar::paint(ui, s, f, w / k)));
    }
    // The rolling sampler's strip between the toolbar and the workspace (issue #82): the browser and the windows
    // begin under it (`work_y`).
    {
        let strip = Rect::new(0.0, work_y(s) - rolling::strip_h(s), w, rolling::strip_h(s));
        if strip.h > 0.0 {
            ui.part(Seg::RollStrip, strip, |ui| rolling::paint(ui, s, f));
        }
    }
    // The floating hint bar, over every window (spec: ui/toolbar-menu.md §4).
    if s.toolbar.hint_bar {
        toolbar::paint_hint_bar(ui, s, f);
    }
    let overlays = ui.hit_count();
    // The toolbar editor's palette goes over the windows, which docked plugin windows leave free like any overlay (spec:
    // ui/toolbar-menu.md §8).
    if s.bar.editing && !s.toolbar.hidden {
        let scale = toolbar_scale_fit(ui.c.scale, w, &s.bar.layout, s.bar.editing);
        ui.scaled(scale, |ui, k| toolbar::paint_palette(ui, s, f, w / k));
    }
    if let Some(m) = s.menu_open {
        ui.scaled(popup_scale(ui.c.scale), |ui, k| menus::paint_dropdown(ui, s, m, k));
    }
    // A menu opened from the settings window (Audio page combos) goes over it; a dropdown's submenu over the dropdown;
    // the time signature dialog's value lists over the dialog.
    let over_dialog = crate::timesig::form_open(s);
    if !s.settings.open && !over_dialog {
        playlist::paint_overlays(ui, s);
    }
    rack::paint_fill(ui, s);
    patterns::paint(ui, s);
    if s.settings.open {
        settings::paint(ui, s);
        if !over_dialog {
            playlist::paint_overlays(ui, s);
        }
    }
    browser::paint_ghost(ui, s);
    if s.monitor.debug {
        mondebug::paint(ui, s);
    }
    if s.cables.borrow().debug {
        cabledebug::paint(ui, s);
    }
    // The Song info box of a project opened with "Show info on opening" (spec: flp/events/project-info.md).
    crate::project_info::paint_song_info(ui, s);
    // The Remote control settings box (spec: ui/remote-control.md §2).
    if let Some(b) = &s.link_box {
        remote_box::paint(ui, s, b);
    }
    // The hover tooltip (crate::tooltip): over the windows and menus, under the message boxes.
    crate::tooltip::paint(ui, s);
    // Message boxes and file dialogs go over everything (spec: ui/dialogs.md).
    dialog::paint(ui, s);
    if over_dialog {
        playlist::paint_overlays(ui, s);
    }
    // Omni Command Palette modal overlay (Ctrl+K).
    palette::paint(ui, s);
    overlays
}

/// Where window `w` paints: its rect (and the channel rack's graph editor hanging below it). The resize zones past
/// its edges are hits only. A repaint that misses this skips the window; a playlist scroll paints around it.
pub fn paint_area(s: &State, w: Win) -> Rect {
    match w {
        Win::ChannelRack => rack::paint_area(s),
        Win::Plugin => channel_win::rect(s),
        Win::Perf => perf::bounds(s),
        Win::Tapper => tapper::bounds(s),
        Win::Touch => touch::bounds(s),
        Win::Strum => strum::bounds(s),
        Win::Arp => arpeggiate::bounds(s),
        Win::PianoDlg => piano_dlg::bounds(s),
        Win::Events => events::bounds(s),
        w if s.is_fixed(w) => s.win_area(w),
        w => s.win_rect(w),
    }
}

/// Start of a Playlist / Mixer / Piano roll window: blocks clicks through it, and (once the
/// content is painted, so the zones sit on top) `resize_zones` adds its resize edges.
pub fn window_body(ui: &mut Ui, r: Rect) {
    ui.hit(r, Action::Swallow, "");
}

/// Resize edges of a floating window: thin strips along the sides and small corner squares,
/// reaching 3 px outside the window. Not for maximized or collapsed windows.
pub fn resize_zones(ui: &mut Ui, s: &State, win: Win, r: Rect) {
    let Some(_) = win.slot() else { return };
    if !s.floats(win) {
        return;
    }
    use crate::app::{EDGE_B, EDGE_L, EDGE_R, EDGE_T};
    let (o, n, c) = (3.0, 2.0, 8.0);
    let hint = "Drag to resize";
    let (x0, x1, y0, y1) = (r.x, r.right(), r.y, r.bottom());
    for (z, e) in [
        (Rect::span(x0 - o, x0 + n, y0 + c, y1 - c), EDGE_L),
        (Rect::span(x1 - n, x1 + o, y0 + c, y1 - c), EDGE_R),
        (Rect::span(x0 + c, x1 - c, y0 - o, y0 + n), EDGE_T),
        (Rect::span(x0 + c, x1 - c, y1 - n, y1 + o), EDGE_B),
        (Rect::span(x0 - o, x0 + c, y0 - o, y0 + c), EDGE_L | EDGE_T),
        (Rect::span(x1 - c, x1 + o, y0 - o, y0 + c), EDGE_R | EDGE_T),
        (Rect::span(x0 - o, x0 + c, y1 - c, y1 + o), EDGE_L | EDGE_B),
        (Rect::span(x1 - c, x1 + o, y1 - c, y1 + o), EDGE_R | EDGE_B),
    ] {
        ui.hit(z, Action::WinResize(win, e), hint);
    }
}

/// Minimize / maximize / close buttons from `x0` at the title bar of a window at top `y`.
pub fn window_buttons(ui: &mut Ui, win: Win, x0: f32, y: f32, _normal: Color, _hover: Color) {
    let mut hovered = None;
    for (i, (hint, action)) in [
        ("Minimize window", Action::MinWin(win)),
        ("Maximize / restore window", Action::MaxWin(win)),
        ("Close window", Action::CloseWin(win)),
    ]
    .into_iter()
    .enumerate()
    {
        if ui.hit(Rect::new(x0 + i as f32 * 19.5, y + 3.0, 17.0, 17.0), action, hint) {
            hovered = Some(i);
        }
    }
    kit::window_buttons(ui.c, ui.t, x0, y, hovered);
}

/// Window frame with a title bar (§5); the bar drags the window. `buttons` = draw the window
/// buttons at the right (the Mixer draws its own over the slot panel). Returns the client rect.
pub fn window_frame(ui: &mut Ui, win: Win, r: Rect, title: &str, title_x: f32, buttons: bool) -> Rect {
    window_frame_ex(ui, win, r, title, title_x, buttons, false)
}

/// `window_frame` with `flat`: a flat title bar without the separator (inside a floating frame).
pub fn window_frame_ex(ui: &mut Ui, win: Win, r: Rect, title: &str, title_x: f32, buttons: bool, flat: bool) -> Rect {
    let t = ui.t;
    ui.c.fill_rect(r, t.window_title);
    let bar = Rect::new(r.x, r.y, r.w, 23.0);
    if kit::lemon::on(t) {
        // Lemon (spec: ui/lemon-design.md §3): a flat bar, a hairline under it.
        if !flat {
            ui.c.fill_rect(Rect::new(r.x, r.y + 22.0, r.w, 1.0), t.splitter_dark);
        }
    } else if !flat {
        ui.c.fill_rect_gradient(bar, t.window_title.shade(0.04), t.window_title.shade(-0.03), 0.0);
        ui.c.fill_rect(Rect::new(r.x, r.y + 22.0, r.w, 1.0), t.splitter_dark);
    }
    ui.c.text(os_ui::Font::new(17), r.x + title_x, r.y + 1.0, t.text, title);
    ui.hit(bar, Action::WinDrag(win), "Drag to move");
    // Minimize / maximize / close glyphs at the right (1862, 1881, 1901 at full width).
    if buttons {
        window_buttons(ui, win, r.right() - 64.0, r.y, t.text_dim.mix(t.glyph, 0.6), t.glyph);
    }
    Rect::new(r.x, r.y + 23.0, r.w, r.h - 23.0)
}

/// Floating window frame (spec ui/piano-roll.md §1a, same for Playlist / Mixer / Piano roll):
/// 1 px dark border, 5 px frame, 1 px inner line around the client, flat 28 px title bar.
/// Returns the inner rect (laid out like the docked window) when `win` floats, else `None`.
pub fn float_frame(ui: &mut Ui, s: &State, win: Win, r: Rect) -> Option<Rect> {
    if !s.floats(win) || r.h < 2.0 * crate::app::TITLE_H {
        return None;
    }
    let t = ui.t;
    let inner = Rect::new(r.x + 7.0, r.y + 5.0, r.w - 14.0, r.h - 12.0);
    if kit::lemon::on(t) {
        kit::float_frame(ui.c, t, r);
    } else {
        ui.c.fill_rect(r, ui.t.s(0x191d1f));
        ui.c.fill_rect(r.inset(1.0), t.window_title);
        ui.c.fill_rect(Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, 1.0), ui.t.s(0x303436));
        ui.c.fill_rect(Rect::new(inner.x - 1.0, inner.y + 22.0, inner.w + 2.0, inner.h - 21.0), ui.t.s(0x131718));
    }
    ui.hit(Rect::new(r.x, r.y, r.w, 28.0), Action::WinDrag(win), "Drag to move");
    Some(inner)
}

/// The (layout rect, client rect) that `framed` returns for `win` at `r`, without painting.
pub fn frame_rects(s: &State, win: Win, r: Rect) -> (Rect, Rect) {
    let floats = s.floats(win) && r.h >= 2.0 * crate::app::TITLE_H;
    let r = if floats { Rect::new(r.x + 7.0, r.y + 5.0, r.w - 14.0, r.h - 12.0) } else { r };
    (r, Rect::new(r.x, r.y + 23.0, r.w, r.h - 23.0))
}

/// Window frame for `win`: the floating frame when it floats, else the docked title bar.
/// Returns (layout rect, client rect).
pub fn framed(ui: &mut Ui, s: &State, win: Win, r: Rect, title: &str, title_x: f32, buttons: bool) -> (Rect, Rect) {
    let buttons = buttons && !s.is_fixed(win);
    match float_frame(ui, s, win, r) {
        Some(inner) => (inner, window_frame_ex(ui, win, inner, title, title_x, buttons, true)),
        None => (r, window_frame(ui, win, r, title, title_x, buttons)),
    }
}

/// "Small scrollbars in editors" is on (General ▸ Miscellaneous; spec: ui/settings-choices.md "Miscellaneous"): the
/// piano roll's and the playlist's scroll bars are about half as thick, without arrow buttons; the room goes to the
/// time ruler and the grid.
pub fn small_scrollbars() -> bool {
    crate::prefs::check(&crate::prefs::SMALL_SCROLLBARS)
}

/// The Playlist picker's scroll bar hint (its right-drag resizes the previews, spec: ui/scroll-bars.md §3).
pub const PICK_BAR_HINT: &str = "Drag to scroll - right-drag resizes the previews";

/// A scroll bar over `track`: thumb sized `view / total`, positioned by `pos` (content pixels).
/// Registers the bar for dragging (see `App::scrollbar_press`).
pub fn scrollbar(ui: &mut Ui, track: Rect, vertical: bool, pos: f32, view: f32, total: f32, target: Scroll) {
    let len = if vertical { track.h } else { track.w };
    let total = total.max(view).max(1.0);
    let max = (total - view).max(0.0);
    let (_, thumb) = os_ui::kit::thumb(len, pos, view, total);
    let hint = match target {
        Scroll::PianoX | Scroll::PlaylistX => "Drag to scroll - right-drag: left/right scrolls, up/down zooms",
        Scroll::PlPick => PICK_BAR_HINT,
        _ => "Drag to scroll",
    };
    let act = Action::Scrollbar(target);
    let hov = ui.hit(track, act.clone(), hint);
    let st = kit::State::new(hov, ui.down(track, &act), false);
    kit::scrollbar(ui.c, ui.t, track, vertical, pos, view, total, st);
    ui.bars.push(Bar { target, track, vertical, max, view, thumb });
}

/// Raised surface: soft vertical gradient (lighter at the top) so panels don't look flat.
pub fn raised(ui: &mut Ui, r: Rect, base: Color, radius: f32) {
    kit::raised_in(ui.c, ui.t, r, base, radius);
}

/// Whether the Lemon design draws (spec: ui/lemon-design.md); views keep their Classic drawing otherwise.
pub fn lemon(ui: &Ui) -> bool {
    kit::lemon::on(ui.t)
}

/// Button face states.
#[derive(Clone, Copy, PartialEq)]
pub enum Look {
    Off,
    Hover,
    On,
    /// Hovered or held window-toggle buttons light up orange (spec: ui/main-window.md §1a).
    Hot,
}

impl Look {
    /// The kit state for this look.
    pub fn state(self) -> kit::State {
        match self {
            Look::Off => kit::State::NORMAL,
            Look::Hover => kit::State::hover(true),
            Look::On => kit::State::new(false, false, true),
            Look::Hot => kit::State::new(true, true, false),
        }
    }
}

/// FL-style raised button face (spec §1): 1 px border with softened corners, an elliptical radial
/// face (light centre, darker edges) and a darker 1 px inner rim.
pub fn button_face(ui: &mut Ui, r: Rect, look: Look) {
    kit::button(ui.c, ui.t, r, look.state());
}

/// A square toolbar button's face for `look` and the colour of its icon. With the Toolbars menu's Flat buttons on
/// (spec: ui/toolbar-menu.md §3) there is no face: a lit button is a slightly darker flat square with a blue icon
/// (Lemon: its button shade, the icon in the brand colour), the others just their icon.
pub fn tool_face(ui: &mut Ui, r: Rect, look: Look) -> Color {
    let lit = matches!(look, Look::On | Look::Hot);
    if !ui.flat {
        button_face(ui, r, look);
    } else if lit && lemon(ui) {
        ui.c.fill_rrect(r.inset(1.0), ui.t.style.radius, ui.t.button);
        return ui.t.brand_lit(os_theme::design::lemon::MAIN);
    } else if lit {
        ui.c.fill_rect(Rect::new(r.x, r.y + 1.0, r.w, r.h - 2.0), ui.t.s(0x282c2e));
        return ui.t.a(0x4589c7);
    }
    match look {
        Look::On | Look::Hot => ui.t.on_glyph,
        Look::Hover => ui.t.tx(0xffffff),
        Look::Off => ui.t.glyph,
    }
}

/// A 30×33 toolbar button (§1). `on` draws the orange "on" state.
/// An `Action::Placeholder` button is not available yet: drawn greyed out, never lit (`Ui::off`).
pub fn tool_button(ui: &mut Ui, r: Rect, icon: Icon, on: bool, action: Action, hint: &'static str) {
    if matches!(action, Action::Placeholder(_)) {
        ui.off(r, hint);
        let st = kit::State::NORMAL.with_disabled(true);
        if !ui.flat {
            kit::button(ui.c, ui.t, r, st);
        }
        icons::draw(ui.c, icon, Rect::new(r.x + 5.0, r.y + 6.5, 20.0, 20.0), kit::ink(ui.t, st));
        return;
    }
    let hov = ui.hit(r, action.clone(), hint);
    let down = ui.down(r, &action);
    let look = if on { Look::On } else if down { Look::Hot } else if hov { Look::Hover } else { Look::Off };
    let c = tool_face(ui, r, look);
    icons::draw(ui.c, icon, Rect::new(r.x + 5.0, r.y + 6.5, 20.0, 20.0), c);
}

/// A sunken panel (hint bar, monitors): flat fill inside a 1 px dark border.
pub fn sunken(ui: &mut Ui, r: Rect, fill: Color) {
    kit::sunken(ui.c, ui.t, r, fill);
}

/// A ring knob (toolbar style): arc from the minimum; bipolar knobs pass `center = true`.
pub fn knob(ui: &mut Ui, cx: f32, cy: f32, r: f32, value: f32, center: bool, _c: Color, _track: Color) {
    let hov = ui.hovering(Rect::new(cx - r, cy - r, 2.0 * r, 2.0 * r));
    kit::knob(ui.c, ui.t, cx, cy, r, value, center, kit::KnobLook::Ring, kit::State::hover(hov));
}

/// A dark glossy ball knob (mixer, rack, slots) with a pointer.
pub fn ball_knob(ui: &mut Ui, cx: f32, cy: f32, r: f32, value: f32) {
    let hov = ui.hovering(Rect::new(cx - r, cy - r, 2.0 * r, 2.0 * r));
    kit::knob(ui.c, ui.t, cx, cy, r, value, false, kit::KnobLook::Ball, kit::State::hover(hov));
}

/// A round LED (spec §7: grey ring, green ball with a bright core when lit).
pub fn led(ui: &mut Ui, cx: f32, cy: f32, on: bool) {
    let hov = ui.hovering(Rect::new(cx - 6.0, cy - 6.0, 12.0, 12.0));
    kit::led(ui.c, ui.t, cx, cy, 6.0, kit::State::hover(hov).with_on(on));
}

/// The small round LED of the mixer's, the playlist's and the channel rack's switches (after FL's), centred on `cx, cy`.
pub fn led_small(ui: &mut Ui, cx: f32, cy: f32, on: bool) {
    let hov = ui.hovering(Rect::new(cx - 6.0, cy - 6.0, 12.0, 12.0));
    kit::led_round(ui.c, ui.t, cx, cy, 4.5, kit::State::hover(hov).with_on(on));
}

/// The label of a question's check that turns its Manage messages entry off when Ok is pressed.
const REMEMBER: &str = "Remember my choice";

/// How wide `remember_check` draws.
pub fn remember_width(ui: &mut Ui) -> f32 {
    20.0 + ui.c.text_width(os_ui::Font::new(13), REMEMBER)
}

/// A question's "Remember my choice" check at the bottom left of its box, left edge `x`, centred on `cy`: a small box,
/// ticked when `on`, and its label; a click on either is `action`. Spec: ui/settings-choices.md "Hints and warnings:
/// Manage messages and Project data size warning (2026-10-16)".
pub fn remember_check(ui: &mut Ui, x: f32, cy: f32, on: bool, action: Action) {
    let t = ui.t;
    let f = os_ui::Font::new(13);
    let w = remember_width(ui);
    let hov = ui.hit(Rect::new(x, cy - 10.0, w, 20.0), action, "Ok with this ticked turns this message off (Manage messages)");
    let b = Rect::new(x, (cy - 6.5).round(), 13.0, 13.0);
    ui.c.fill_rrect(b, 2.0, t.s(0x141516));
    ui.c.stroke_rrect(b, 2.0, 1.0, if hov { t.glyph_dim } else { t.text_dim });
    if on {
        kit::tick(ui.c, b, t.glyph);
    }
    let lh = ui.c.line_height(f);
    ui.c.text(f, x + 20.0, (cy - lh / 2.0).round(), if hov { t.glyph } else { t.text }, REMEMBER);
}

/// Channel / clip colors from the model.
pub fn rgb(c: os_model::Rgb) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a: 255 }
}
