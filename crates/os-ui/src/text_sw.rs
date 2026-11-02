// Copyright (C) 2026 ItsAlphaNeon
// Copyright (C) 2026 thacer13
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Text on Linux and macOS: outlines through swash (TrueType hinting, quarter-pixel pen positions, blending in a
//! gamma of 1.8) in the Barlow family (SIL OFL 1.1, see `fonts/Barlow/OFL.txt`), which stays crisp at the
//! fractional scales Wayland and X11 desktops run at.
//!
//! It keeps the layout model of the portable engine (Cuprum's cell height and baseline, `Font::width` as
//! an average character width), so the layouts measured for Cuprum hold: Barlow is sized so that its
//! capitals are as tall as Cuprum's and fitted to Cuprum's average width. Bold text uses the family's
//! semibold, strongly condensed text its narrow cut, and digits are tabular so a counting number keeps
//! its width. `OPENSTUDIO_TEXT=cuprum` brings back the portable engine with Cuprum.
//!
//! Text Barlow can't show (file names in any language) takes the slower way: put in visual order (`unicode-bidi`), each
//! part in the face that has it (Noto Sans Arabic, bundled, then whatever the system offers) and shaped by swash, so
//! Arabic letters join.

use std::collections::HashMap;

use os_theme::Color;
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::{Direction, ShapeContext};
use swash::text::{Codepoint, Script};
use swash::zeno::{Format, Transform, Vector};
use swash::FontRef;
use tiny_skia::Pixmap;

use super::{ClipPx, Face, Font, CUPRUM};

static REGULAR: &[u8] = include_bytes!("../fonts/Barlow/BarlowSemiCondensed-Regular.ttf");
static SEMIBOLD: &[u8] = include_bytes!("../fonts/Barlow/BarlowSemiCondensed-SemiBold.ttf");
static NARROW: &[u8] = include_bytes!("../fonts/Barlow/BarlowCondensed-Medium.ttf");
static ARABIC: &[u8] = include_bytes!("../fonts/NotoSansArabic/NotoSansArabic-Regular.ttf");
/// The space text is blended in.
const GAMMA: f32 = 1.8;

/// Segoe UI's baseline (see the portable engine).
const SEGOE_UI: (f32, f32) = (2210.0 / 2048.0, 514.0 / 2048.0);
/// Synthetic italic slant.
const SLANT: f32 = 0.21;
/// Positions per pixel a glyph is rendered at.
const PHASES: f32 = 4.0;
/// Shaped strings kept: past this the cache starts over.
const SHAPED_MAX: usize = 4096;

struct Glyph {
    cov: Vec<u8>,
    w: usize,
    h: usize,
    left: i32,
    top: i32,
}

struct FaceData {
    font: FontRef<'static>,
    /// Em factor that makes this face's capitals as tall as Cuprum's.
    size_k: f32,
    /// Average advance (em, after `size_k`) of the fitting sample at the default axis values.
    avg: f32,
}

/// One glyph of a shaped string: its face (`Sw::face`'s numbering), and where it goes in px from the string's start.
#[derive(Clone, Copy)]
struct Placed {
    face: u16,
    id: u16,
    x: f32,
    y: f32,
}

/// A string laid out by `Sw::shape`, in visual order.
struct Shaped {
    w: f32,
    glyphs: Vec<Placed>,
}

pub struct Sw {
    ctx: ScaleContext,
    shaper: ShapeContext,
    regular: FaceData,
    bold: FaceData,
    narrow: FaceData,
    /// Faces for what the three above don't have, numbered from `FIRST_FALLBACK`: the bundled Arabic, then what the
    /// system gave, each loaded when first needed.
    fallback: Vec<FaceData>,
    /// System fonts already loaded (file, index), and the characters no font was found for: neither is asked again.
    loaded: Vec<(String, u32)>,
    missing: std::collections::HashSet<char>,
    /// Cuprum's win ascent / descent and average character width (em): the layout's reference.
    ui: (f32, f32),
    ref_avg: f32,
    /// Cuprum's average advance over the fitting sample (em).
    ref_sample: f32,
    to_lin: [u16; 256],
    from_lin: Vec<u8>,
    glyphs: HashMap<(char, Font, u32, u8), Glyph>,
    advances: HashMap<(char, Font, u32), f32>,
    shaped: HashMap<(Box<str>, Font, u32), std::rc::Rc<Shaped>>,
    shaped_glyphs: HashMap<(u16, u16, Font, u32, u8), Glyph>,
}

/// Text the widths are fitted on: the mix of letters and digits the UI's labels have.
const SAMPLE: &str = "Channel rack Mixer Playlist Insert 12 Master Volume 100% Pattern 3 Tempo 140.000";

fn face(data: &'static [u8], cap_ref: f32) -> Result<FaceData, String> {
    let font = FontRef::from_index(data, 0).ok_or("not a font swash can read")?;
    let m = font.metrics(&[]);
    let upem = f32::from(m.units_per_em).max(1.0);
    let cap = if m.cap_height > 0.0 { m.cap_height / upem } else { 0.7 };
    let size_k = if cap_ref > 0.0 { cap_ref / cap } else { 1.0 };
    let gm = font.glyph_metrics(&[]);
    let cm = font.charmap();
    let total: f32 = SAMPLE.chars().map(|c| gm.advance_width(cm.map(c))).sum();
    let avg = total / upem / SAMPLE.chars().count() as f32 * size_k;
    Ok(FaceData { font, size_k, avg })
}

/// Faces an engine holds (regular, semibold, narrow): its fonts in the CPU panel's Misc reading.
const FACES: usize = 3;
/// `Sw::face`'s number of the first fallback.
const FIRST_FALLBACK: u16 = FACES as u16;

/// Whether `c` belongs to right-to-left writing: such text needs reordering and, for Arabic, joining.
fn right_to_left(c: char) -> bool {
    matches!(u32::from(c), 0x590..=0x8ff | 0xfb1d..=0xfdff | 0xfe70..=0xfeff | 0x10800..=0x10fff | 0x1e800..=0x1efff)
}

/// A system font with `c` in it, as (file, index in the file).
#[cfg(target_os = "macos")]
fn system_font(_c: char) -> Vec<(String, u32)> {
    ["Supplemental/Arial Unicode.ttf", "PingFang.ttc", "Apple Symbols.ttf", "Supplemental/Tahoma.ttf"]
        .iter()
        .map(|f| (format!("/System/Library/Fonts/{f}"), 0))
        .collect()
}

/// A system font with `c` in it, as (file, index in the file): fontconfig's pick.
#[cfg(not(target_os = "macos"))]
fn system_font(c: char) -> Vec<(String, u32)> {
    let pattern = format!("sans:charset={:x}", u32::from(c));
    let Ok(out) = std::process::Command::new("fc-match").args(["-f", "%{file}|%{index}", &pattern]).output() else { return Vec::new() };
    let text = String::from_utf8_lossy(&out.stdout);
    text.rsplit_once('|').map(|(file, index)| (file.to_string(), index.parse().unwrap_or(0))).into_iter().collect()
}

impl Drop for Sw {
    fn drop(&mut self) {
        super::FONTS.fetch_sub(FACES, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Sw {
    pub fn new() -> Result<Sw, String> {
        let (asc, desc, ref_avg) = super::portable::os2(CUPRUM).ok_or("the UI font has no OS/2 metrics")?;
        let cuprum = face(CUPRUM, 0.0)?;
        let cm = cuprum.font.metrics(&[]);
        let cap_ref = cm.cap_height / f32::from(cm.units_per_em);
        let mut to_lin = [0u16; 256];
        for (i, v) in to_lin.iter_mut().enumerate() {
            *v = ((i as f32 / 255.0).powf(GAMMA) * 4095.0).round() as u16;
        }
        let from_lin = (0..4096).map(|i| ((i as f32 / 4095.0).powf(1.0 / GAMMA) * 255.0).round() as u8).collect();
        let sw = Sw {
            ctx: ScaleContext::new(),
            shaper: ShapeContext::new(),
            ref_sample: cuprum.avg,
            regular: face(REGULAR, cap_ref)?,
            bold: face(SEMIBOLD, cap_ref)?,
            narrow: face(NARROW, cap_ref)?,
            fallback: vec![face(ARABIC, cap_ref)?],
            loaded: Vec::new(),
            missing: Default::default(),
            ui: (asc, desc),
            ref_avg,
            to_lin,
            from_lin,
            glyphs: HashMap::new(),
            advances: HashMap::new(),
            shaped: HashMap::new(),
            shaped_glyphs: HashMap::new(),
        };
        super::FONTS.fetch_add(FACES, std::sync::atomic::Ordering::Relaxed);
        Ok(sw)
    }

    /// The em height in device pixels (rounded, as the layouts expect).
    fn px(f: Font, scale: f32) -> f32 {
        (f32::from(f.size) * scale).round().max(1.0)
    }

    fn vm(&self, f: Font) -> (f32, f32) {
        match f.face {
            Face::Ui => self.ui,
            Face::Light => SEGOE_UI,
        }
    }

    /// How a font request is set up: which face (0 regular, 1 bold, 2 narrow) and the horizontal factor
    /// left for the outline transform.
    fn setup(&self, f: Font, scale: f32) -> (u8, f32) {
        let mut which = u8::from(f.bold);
        let fd = self.face(u16::from(which));
        // Horizontal factor wanted against the face's own width.
        let mut k = if f.width != 0 {
            let want = (f32::from(f.width) * scale).round().max(1.0);
            (want / (self.ref_avg * Self::px(f, scale))).clamp(0.3, 2.0) * self.ref_sample / fd.avg
        } else {
            self.ref_sample / fd.avg
        };
        // A strong squeeze thins the stems with the letters: the narrow cut is drawn for that width.
        if !f.bold && k < 0.9 {
            k *= fd.avg / self.narrow.avg;
            which = 2;
        }
        (which, k)
    }

    fn face(&self, which: u16) -> &FaceData {
        match which {
            1 => &self.bold,
            2 => &self.narrow,
            n if n >= FIRST_FALLBACK => &self.fallback[usize::from(n - FIRST_FALLBACK)],
            _ => &self.regular,
        }
    }

    /// Whether `s` needs `shape`: right-to-left text, or a character Barlow doesn't have.
    fn complex(&self, s: &str) -> bool {
        !s.is_ascii() && s.chars().any(|c| !c.is_ascii() && (right_to_left(c) || (!c.is_control() && self.regular.font.charmap().map(c) == 0)))
    }

    /// The face to show `c` with, `own` if it can; loads a system font when none of those held has it.
    fn face_for(&mut self, c: char, own: u16) -> u16 {
        let has = |fd: &FaceData| fd.font.charmap().map(c) != 0;
        if has(self.face(own)) {
            return own;
        }
        if let Some(i) = self.fallback.iter().position(has) {
            return FIRST_FALLBACK + i as u16;
        }
        if self.missing.contains(&c) {
            return own;
        }
        for (file, index) in system_font(c) {
            if self.loaded.contains(&(file.clone(), index)) {
                continue;
            }
            self.loaded.push((file.clone(), index));
            let Ok(data) = std::fs::read(&file) else { continue };
            // Kept for the program's life, as the bundled faces are.
            let data: &'static [u8] = Box::leak(data.into_boxed_slice());
            let Some(font) = FontRef::from_index(data, index as usize) else { continue };
            let fd = FaceData { font, size_k: 1.0, avg: 1.0 };
            let found = has(&fd);
            self.fallback.push(fd);
            super::FONTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if found {
                return FIRST_FALLBACK + self.fallback.len() as u16 - 1;
            }
        }
        self.missing.insert(c);
        own
    }

    /// The em size and horizontal factor of a face for `f`: a fallback's letters at the size of Barlow's, unsqueezed.
    fn face_size(&self, which: u16, f: Font, scale: f32, k: f32) -> (f32, f32) {
        let px = Self::px(f, scale);
        if which >= FIRST_FALLBACK { (px * self.regular.size_k, 1.0) } else { (px * self.face(which).size_k, k) }
    }

    /// `s` laid out: in visual order, each part in the face that has it, shaped.
    fn shape(&mut self, s: &str, f: Font, scale: f32) -> std::rc::Rc<Shaped> {
        let key = (Box::from(s), f, scale.to_bits());
        if let Some(l) = self.shaped.get(&key) {
            return l.clone();
        }
        let (own, k) = self.setup(f, scale);
        let own = u16::from(own);
        let mut glyphs = Vec::new();
        let mut pen = 0.0_f32;
        let bidi = unicode_bidi::BidiInfo::new(s, None);
        for para in &bidi.paragraphs {
            let (levels, runs) = bidi.visual_runs(para, para.range.clone());
            for run in runs {
                let rtl = levels[run.start].is_rtl();
                // The run cut where the face or the script changes; neutral characters stay with what's before them.
                let mut items: Vec<(usize, usize, u16, Script)> = Vec::new();
                for (i, c) in s[run.clone()].char_indices() {
                    let i = i + run.start;
                    let script = c.script();
                    let neutral = matches!(script, Script::Common | Script::Inherited | Script::Unknown);
                    match items.last_mut() {
                        Some(last) if neutral && self.face(last.2).font.charmap().map(c) != 0 => last.1 = i + c.len_utf8(),
                        Some(last) if !neutral && last.3 == script && self.face(last.2).font.charmap().map(c) != 0 => last.1 = i + c.len_utf8(),
                        Some(last) if !neutral && matches!(last.3, Script::Common | Script::Inherited | Script::Unknown) && self.face(last.2).font.charmap().map(c) != 0 => {
                            last.3 = script;
                            last.1 = i + c.len_utf8();
                        }
                        _ => {
                            let face = self.face_for(c, own);
                            items.push((i, i + c.len_utf8(), face, script));
                        }
                    }
                }
                if rtl {
                    items.reverse();
                }
                for (a, b, face, script) in items {
                    let (size, hk) = self.face_size(face, f, scale, k);
                    let font = self.face(face).font;
                    let dir = if rtl { Direction::RightToLeft } else { Direction::LeftToRight };
                    let mut shaper = self.shaper.builder(font).script(script).direction(dir).size(size).build();
                    shaper.add_str(&s[a..b]);
                    // Clusters come in logical order: a right-to-left run reads back from its end.
                    let mut clusters: Vec<Vec<Placed>> = Vec::new();
                    let mut advances: Vec<f32> = Vec::new();
                    shaper.shape_with(|cl| {
                        let mut at = 0.0;
                        let mut part = Vec::with_capacity(cl.glyphs.len());
                        for g in cl.glyphs {
                            part.push(Placed { face, id: g.id, x: at + g.x * hk, y: g.y });
                            at += g.advance * hk;
                        }
                        clusters.push(part);
                        advances.push(at);
                    });
                    if rtl {
                        clusters.reverse();
                        advances.reverse();
                    }
                    for (part, adv) in clusters.into_iter().zip(advances) {
                        glyphs.extend(part.into_iter().map(|p| Placed { x: pen + p.x, ..p }));
                        pen += adv;
                    }
                }
            }
        }
        if self.shaped.len() >= SHAPED_MAX {
            self.shaped.clear();
        }
        let line = std::rc::Rc::new(Shaped { w: pen, glyphs });
        self.shaped.insert(key, line.clone());
        line
    }

    /// Render a shaped string's glyph, once.
    fn shaped_glyph(&mut self, p: Placed, f: Font, scale: f32, phase: u8) {
        let key = (p.face, p.id, f, scale.to_bits(), phase);
        if self.shaped_glyphs.contains_key(&key) {
            return;
        }
        let (_, k) = self.setup(f, scale);
        let (size, hk) = self.face_size(p.face, f, scale, k);
        let font = self.face(p.face).font;
        let mut scaler = self.ctx.builder(font).size(size).hint(p.face < FIRST_FALLBACK).build();
        let mut r = Render::new(&[Source::Outline]);
        r.format(Format::Alpha);
        r.offset(Vector::new(f32::from(phase) / PHASES, 0.0));
        let slant = if f.italic { SLANT } else { 0.0 };
        if (hk - 1.0).abs() > 1e-3 || f.italic {
            r.transform(Some(Transform::new(hk, 0.0, slant, 1.0, 0.0, 0.0)));
        }
        // These faces come in one weight.
        if f.bold && p.face >= FIRST_FALLBACK {
            r.embolden(size * 0.03);
        }
        let g = match r.render(&mut scaler, p.id) {
            Some(img) => Glyph { w: img.placement.width as usize, h: img.placement.height as usize, left: img.placement.left, top: img.placement.top, cov: img.data },
            None => Glyph { cov: Vec::new(), w: 0, h: 0, left: 0, top: 0 },
        };
        self.shaped_glyphs.insert(key, g);
    }

    /// Digits are tabular: each takes the widest digit's advance and sits in its middle, so a number that
    /// counts does not change its width. Returns (advance, offset of the glyph inside it).
    fn advance(&mut self, c: char, f: Font, scale: f32) -> (f32, f32) {
        let own = self.own_advance(c, f, scale);
        if !c.is_ascii_digit() {
            return (own, 0.0);
        }
        let widest = ('0'..='9').map(|d| self.own_advance(d, f, scale)).fold(0.0, f32::max);
        (widest, (widest - own) * 0.5)
    }

    fn own_advance(&mut self, c: char, f: Font, scale: f32) -> f32 {
        let key = (c, f, scale.to_bits());
        if let Some(a) = self.advances.get(&key) {
            return *a;
        }
        let (which, k) = self.setup(f, scale);
        let fd = self.face(u16::from(which));
        let gm = fd.font.glyph_metrics(&[]).scale(Self::px(f, scale) * fd.size_k);
        let a = gm.advance_width(fd.font.charmap().map(c)) * k;
        self.advances.insert(key, a);
        a
    }

    fn glyph(&mut self, c: char, f: Font, scale: f32, phase: u8) -> &Glyph {
        let key = (c, f, scale.to_bits(), phase);
        if !self.glyphs.contains_key(&key) {
            let (which, k) = self.setup(f, scale);
            let fd = match which {
                1 => &self.bold,
                2 => &self.narrow,
                _ => &self.regular,
            };
            let px = Self::px(f, scale) * fd.size_k;
            // The light face (the song position digits) has no cut of its own here: the regular stands in.
            let mut scaler = self.ctx.builder(fd.font).size(px).hint(true).build();
            let mut r = Render::new(&[Source::Outline]);
            r.format(Format::Alpha);
            r.offset(Vector::new(f32::from(phase) / PHASES, 0.0));
            let slant = if f.italic { SLANT } else { 0.0 };
            if (k - 1.0).abs() > 1e-3 || f.italic {
                r.transform(Some(Transform::new(k, 0.0, slant, 1.0, 0.0, 0.0)));
            }
            let g = match r.render(&mut scaler, fd.font.charmap().map(c)) {
                Some(img) => Glyph {
                    w: img.placement.width as usize,
                    h: img.placement.height as usize,
                    left: img.placement.left,
                    top: img.placement.top,
                    cov: img.data,
                },
                None => Glyph { cov: Vec::new(), w: 0, h: 0, left: 0, top: 0 },
            };
            self.glyphs.insert(key, g);
        }
        &self.glyphs[&key]
    }

    pub fn metrics(&mut self, f: Font, scale: f32) -> (i32, i32) {
        let px = Self::px(f, scale);
        let (a, d) = self.vm(f);
        let (asc, desc) = ((a * px).round() as i32, (d * px).round() as i32);
        (asc + desc, asc)
    }

    pub fn extent(&mut self, f: Font, scale: f32, s: &str) -> (i32, i32) {
        if self.complex(s) {
            let w = self.shape(s, f, scale).w;
            return (w.round() as i32, self.metrics(f, scale).0);
        }
        let w: f32 = s.chars().map(|c| self.advance(c, f, scale).0).sum();
        (w.round() as i32, self.metrics(f, scale).0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, pix: &mut Pixmap, clip: ClipPx, f: Font, scale: f32, x: i32, y: i32, c: Color, s: &str, vertical: bool) {
        let asc = self.metrics(f, scale).1;
        let col = [c.r, c.g, c.b].map(|v| u32::from(self.to_lin[usize::from(v)]));
        if self.complex(s) {
            let line = self.shape(s, f, scale);
            for &p in &line.glyphs {
                let q = (p.x * PHASES).round() as i32;
                let (whole, phase) = (q.div_euclid(PHASES as i32), q.rem_euclid(PHASES as i32) as u8);
                self.shaped_glyph(p, f, scale, phase);
                let g = &self.shaped_glyphs[&(p.face, p.id, f, scale.to_bits(), phase)];
                let at = (whole + g.left, asc - g.top - p.y.round() as i32);
                blit(g, at, &self.to_lin, &self.from_lin, pix, clip, (x, y), col, c.a, vertical);
            }
            return;
        }
        let mut pen = 0.0f32;
        for chr in s.chars() {
            let (adv, inset) = self.advance(chr, f, scale);
            let q = ((pen + inset) * PHASES).round() as i32;
            let (whole, phase) = (q.div_euclid(PHASES as i32), q.rem_euclid(PHASES as i32) as u8);
            self.glyph(chr, f, scale, phase);
            let g = &self.glyphs[&(chr, f, scale.to_bits(), phase)];
            let at = (whole + g.left, asc - g.top);
            blit(g, at, &self.to_lin, &self.from_lin, pix, clip, (x, y), col, c.a, vertical);
            pen += adv;
        }
    }
}

/// Blend a glyph's coverage into `pix`, its corner `at` from the string's origin `xy`.
#[allow(clippy::too_many_arguments)]
fn blit(g: &Glyph, at: (i32, i32), to_lin: &[u16; 256], from_lin: &[u8], pix: &mut Pixmap, clip: ClipPx, xy: (i32, i32), col: [u32; 3], alpha: u8, vertical: bool) {
    let pw = pix.width() as i32;
    let data = pix.data_mut();
    for j in 0..g.h {
        for i in 0..g.w {
            let a = u32::from(g.cov[j * g.w + i]) * u32::from(alpha) / 255;
            if a == 0 {
                continue;
            }
            let (lx, ly) = (at.0 + i as i32, at.1 + j as i32);
            let (dx, dy) = if vertical { (xy.0 + ly, xy.1 - lx) } else { (xy.0 + lx, xy.1 + ly) };
            if dx < clip.0 || dy < clip.1 || dx >= clip.2 || dy >= clip.3 {
                continue;
            }
            let kk = ((dy * pw + dx) * 4) as usize;
            for (n, sv) in col.into_iter().enumerate() {
                let dv = u32::from(to_lin[usize::from(data[kk + n])]);
                data[kk + n] = from_lin[((sv * a + dv * (255 - a)) / 255) as usize];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_is_joined_and_right_to_left() {
        let mut e = Sw::new().unwrap();
        let f = Font::new(14);
        let word = "\u{0645}\u{062d}\u{0645}\u{062f}";
        assert!(e.complex(word));
        assert!(!e.complex("Kick 12 caf\u{e9}"), "Barlow has these");
        let line = e.shape(word, f, 1.0);
        assert!(line.w > 0.0 && line.glyphs.iter().all(|g| g.id != 0 && g.face >= FIRST_FALLBACK), "real glyphs from the Arabic face");
        // Joined forms differ from the letters alone: the first and third letter are one character in two shapes.
        assert_ne!(line.glyphs.first().unwrap().id, line.glyphs[2].id);
        assert!(line.glyphs.windows(2).all(|p| p[0].x <= p[1].x), "left to right on screen");
    }

    #[test]
    fn mixed_text_keeps_each_face_and_the_order_on_screen() {
        let mut e = Sw::new().unwrap();
        let line = e.shape("Song \u{0645}\u{062d}\u{0645}\u{062f}.wav", Font::new(14), 1.0);
        assert_eq!(line.glyphs[0].face, 0, "starts with the Latin word");
        assert!(line.glyphs.iter().any(|g| g.face >= FIRST_FALLBACK));
        assert_eq!(line.glyphs.last().unwrap().face, 0, "ends with the extension");
    }
}
