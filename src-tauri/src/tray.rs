//! Menu bar / system tray icon: glyph, live count, tooltip and the
//! right-click menu. Left click toggles the popover.
//!
//! Contract:
//! - macOS: template glyph (`tray-black.png`, auto light/dark); the running
//!   count is the tray title when `show_count_in_tray`. Attention (a session
//!   needs the user) adds a filled dot at the top-right of the glyph — a
//!   badge *shape*, since template icons are monochrome.
//! - Windows/Linux: white glyph at 32×32 with the count (1–9, "9+") drawn
//!   into the bottom-right corner from an embedded 5×7 pixel font when
//!   `show_count_in_tray`, plus an orange attention dot.
//! - Busy vs. idle sessions do not change the glyph: the count already says
//!   "something is running" and the finer state belongs to the popover and
//!   the panel. A "filled" busy variant was deliberately not added because it
//!   cannot be visually verified in this environment; `IconKey` is the place
//!   to add it.
//! - `update` runs on every publish, from any thread: it is diffed against
//!   what was last applied, so the tray is only touched when something
//!   changed, and composed icons are cached per key.
//! - Linux app indicators do not report clicks at all: the menu opens on
//!   either button and "Open Heron" is the way to the popover there.

use crate::model::Snapshot;
use crate::{state, windows};
use image::{ImageFormat, RgbaImage};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::OnceLock;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

pub const TRAY_ID: &str = "main";

#[cfg(target_os = "macos")]
const GLYPH: &[u8] = include_bytes!("../icons/tray/tray-black.png");
#[cfg(not(target_os = "macos"))]
const GLYPH: &[u8] = include_bytes!("../icons/tray/tray-white.png");

/// Everything that decides which icon is shown.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct IconKey {
    /// 0 = no badge, 1–9, 10 = "9+". Always 0 on macOS (the title carries it).
    count: u8,
    attention: bool,
}

/// What was last pushed to the tray, to skip redundant (main-thread) calls.
#[derive(Default)]
struct Applied {
    icon: Option<IconKey>,
    title: Option<String>,
    tooltip: Option<String>,
    panel_shown: Option<bool>,
}

static PANEL_ITEM: OnceLock<MenuItem<Wry>> = OnceLock::new();
static ICONS: OnceLock<Mutex<HashMap<IconKey, Image<'static>>>> = OnceLock::new();
static APPLIED: Mutex<Applied> = Mutex::new(Applied { icon: None, title: None, tooltip: None, panel_shown: None });

fn panel_label(shown: bool) -> &'static str {
    if shown {
        "Hide Panel"
    } else {
        "Show Panel"
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show_panel = app.state::<state::AppState>().settings().show_panel;
    let open = MenuItemBuilder::with_id("open", "Open Heron").build(app)?;
    let new_session =
        MenuItemBuilder::with_id("new", "New Session…").accelerator("CmdOrCtrl+N").build(app)?;
    let panel = MenuItemBuilder::with_id("panel", panel_label(show_panel)).build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings…").accelerator("CmdOrCtrl+,").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Heron").accelerator("CmdOrCtrl+Q").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &open,
            &new_session,
            &panel,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ])
        .build()?;
    let _ = PANEL_ITEM.set(panel);

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(IconKey { count: 0, attention: false }))
        .icon_as_template(true)
        .tooltip("Heron")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => windows::show_popover(app),
            "new" => windows::new_session_with_picker(app),
            "panel" => windows::toggle_panel(app),
            "settings" => windows::open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            match &event {
                TrayIconEvent::Click { rect, .. }
                | TrayIconEvent::Enter { rect, .. }
                | TrayIconEvent::Move { rect, .. }
                | TrayIconEvent::Leave { rect, .. } => windows::note_tray_rect(rect),
                _ => {}
            }
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_popover(tray.app_handle());
            }
        })
        .build(app)?;

    let snap = app.state::<state::AppState>().snapshot();
    update(app, &snap);
    Ok(())
}

/// Reflect the latest snapshot in the tray (icon variant, count, tooltip,
/// panel menu label). Cheap: everything is diffed against the last call.
pub fn update(app: &AppHandle, snap: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let running = snap.running_count();
    let attention = snap.attention_count();
    let show_count = snap.settings.show_count_in_tray && running > 0;
    let macos = cfg!(target_os = "macos");

    let key = IconKey {
        count: if show_count && !macos { running.min(10) as u8 } else { 0 },
        attention: attention > 0,
    };
    // An empty title clears it; the tray-icon crate ignores `None`.
    let title = if show_count && macos { running.to_string() } else { String::new() };
    let tooltip = match (running, attention) {
        (0, _) => "Heron — no Claude sessions".to_string(),
        (n, 0) => format!("Heron — {n} running"),
        (n, 1) => format!("Heron — {n} running, 1 needs you"),
        (n, a) => format!("Heron — {n} running, {a} need you"),
    };
    let panel_shown = snap.settings.show_panel;

    // Decide under the lock, apply outside it: tray setters hop to the main thread.
    let (icon_changed, title_changed, tooltip_changed, panel_changed) = {
        let mut a = APPLIED.lock();
        let changed = (
            a.icon != Some(key),
            a.title.as_deref() != Some(title.as_str()),
            a.tooltip.as_deref() != Some(tooltip.as_str()),
            a.panel_shown != Some(panel_shown),
        );
        a.icon = Some(key);
        a.title = Some(title.clone());
        a.tooltip = Some(tooltip.clone());
        a.panel_shown = Some(panel_shown);
        changed
    };
    if icon_changed {
        let _ = tray.set_icon(Some(icon_for(key)));
    }
    if title_changed {
        let _ = tray.set_title(Some(&title));
    }
    if tooltip_changed {
        let _ = tray.set_tooltip(Some(&tooltip));
    }
    if panel_changed {
        if let Some(item) = PANEL_ITEM.get() {
            let _ = item.set_text(panel_label(panel_shown));
        }
    }
}

// ---- icon composition -------------------------------------------------------

fn icon_for(key: IconKey) -> Image<'static> {
    let cache = ICONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(img) = cache.lock().get(&key) {
        return img.clone();
    }
    let img = compose(key);
    cache.lock().insert(key, img.clone());
    img
}

fn compose(key: IconKey) -> Image<'static> {
    let mut canvas = base_glyph();
    if key.count > 0 {
        badge::count(&mut canvas, key.count);
    }
    if key.attention {
        let (w, _) = canvas.dimensions();
        // Top-right corner; black on macOS (template), orange elsewhere.
        if cfg!(target_os = "macos") {
            let s = w as f64 / 64.0;
            badge::dot(&mut canvas, 54.0 * s, 10.0 * s, 7.0 * s, 3.0 * s, [0, 0, 0, 255]);
        } else {
            let s = w as f64 / 32.0;
            badge::dot(&mut canvas, 26.5 * s, 5.5 * s, 4.5 * s, 1.5 * s, [255, 159, 10, 255]);
        }
    }
    let (w, h) = canvas.dimensions();
    Image::new_owned(canvas.into_raw(), w, h)
}

/// The shipped glyph: 64×64 on macOS (AppKit scales it to 18 pt), resized to
/// 32×32 elsewhere so badges are drawn at the size the taskbar shows.
fn base_glyph() -> RgbaImage {
    static BASE: OnceLock<RgbaImage> = OnceLock::new();
    BASE.get_or_init(|| {
        let decoded = match image::load_from_memory_with_format(GLYPH, ImageFormat::Png) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                log::warn!("embedded tray glyph unreadable: {e}");
                RgbaImage::new(32, 32)
            }
        };
        if cfg!(target_os = "macos") {
            decoded
        } else {
            image::imageops::resize(&decoded, 32, 32, image::imageops::FilterType::Triangle)
        }
    })
    .clone()
}

mod badge {
    use image::{Rgba, RgbaImage};
    use std::collections::HashSet;

    /// A filled disc with a transparent ring around it, so the dot reads as
    /// its own shape even where it overlaps the glyph (and on a template icon
    /// that has no colour to tell them apart). Edges are anti-aliased.
    pub fn dot(img: &mut RgbaImage, cx: f64, cy: f64, r: f64, ring: f64, color: [u8; 4]) {
        let (w, h) = img.dimensions();
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                if d > r + ring + 0.5 {
                    continue;
                }
                let cleared = (r + ring + 0.5 - d).clamp(0.0, 1.0);
                let filled = (r + 0.5 - d).clamp(0.0, 1.0);
                let p = img.get_pixel_mut(x, y);
                let bg_alpha = p[3] as f64 * (1.0 - cleared);
                let dot_alpha = color[3] as f64 * filled;
                if dot_alpha >= bg_alpha {
                    *p = Rgba([color[0], color[1], color[2], dot_alpha.round() as u8]);
                } else {
                    p[3] = bg_alpha.round() as u8;
                }
            }
        }
    }

    /// 5×7 digits, one byte per row, MSB = left column.
    const DIGITS: [[u8; 7]; 10] = [
        [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
    ];
    /// A compact 3-wide plus for "9+".
    const PLUS: [u8; 7] = [0b000, 0b000, 0b010, 0b111, 0b010, 0b000, 0b000];
    const SCALE: u32 = 2;

    /// Draw `count` (1–9, anything else = "9+") in the bottom-right corner at
    /// 2× (10×14 px per digit on the 32×32 canvas, so still legible once the
    /// taskbar shows it at 16×16), white with a dark 1-px outline.
    pub fn count(img: &mut RgbaImage, count: u8) {
        let (w, h) = img.dimensions();
        let glyphs: Vec<(u32, [u8; 7])> = if (1..=9).contains(&count) {
            vec![(5, DIGITS[count as usize])]
        } else {
            vec![(5, DIGITS[9]), (3, PLUS)]
        };
        let total: u32 = glyphs.iter().map(|g| g.0 * SCALE).sum::<u32>() + (glyphs.len() as u32 - 1) * SCALE;
        let mut x0 = w as i32 - total as i32 - 1;
        let y0 = h as i32 - (7 * SCALE) as i32 - 1;

        let mut on: Vec<(i32, i32)> = Vec::new();
        for (width, rows) in &glyphs {
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..*width {
                    if (bits >> (width - 1 - col)) & 1 == 1 {
                        for sy in 0..SCALE {
                            for sx in 0..SCALE {
                                on.push((x0 + (col * SCALE + sx) as i32, y0 + (row as u32 * SCALE + sy) as i32));
                            }
                        }
                    }
                }
            }
            x0 += ((width + 1) * SCALE) as i32;
        }

        let set: HashSet<(i32, i32)> = on.iter().copied().collect();
        for &(x, y) in &on {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let p = (x + dx, y + dy);
                    if !set.contains(&p) {
                        put(img, p, Rgba([0, 0, 0, 190]));
                    }
                }
            }
        }
        for &(x, y) in &on {
            put(img, (x, y), Rgba([255, 255, 255, 255]));
        }
    }

    fn put(img: &mut RgbaImage, (x, y): (i32, i32), color: Rgba<u8>) {
        let (w, h) = img.dimensions();
        if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
            img.put_pixel(x as u32, y as u32, color);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn dot_fills_centre_and_clears_ring() {
            let mut img = RgbaImage::from_pixel(32, 32, Rgba([255, 255, 255, 255]));
            dot(&mut img, 16.0, 16.0, 4.0, 2.0, [255, 159, 10, 255]);
            assert_eq!(img.get_pixel(16, 16).0, [255, 159, 10, 255]);
            assert_eq!(img.get_pixel(16, 21).0[3], 0, "ring is transparent");
            assert_eq!(img.get_pixel(16, 25).0, [255, 255, 255, 255], "outside untouched");
        }

        #[test]
        fn count_draws_white_pixels_with_outline_in_the_corner() {
            let mut img = RgbaImage::new(32, 32);
            count(&mut img, 1);
            // The "1" has a foot spanning three glyph columns at the bottom row.
            let white = |x, y| img.get_pixel(x, y).0 == [255, 255, 255, 255];
            assert!(white(24, 30) && white(26, 30));
            assert_eq!(img.get_pixel(24, 31).0, [0, 0, 0, 190], "outline below");
            assert_eq!(img.get_pixel(2, 2).0[3], 0, "top-left untouched");
        }

        #[test]
        fn nine_plus_fits_the_canvas() {
            let mut img = RgbaImage::new(32, 32);
            count(&mut img, 10);
            let lit = (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).filter(|&(x, y)| img.get_pixel(x, y).0[3] > 0);
            let min_x = lit.map(|(x, _)| x).min().unwrap_or(32);
            assert!(min_x >= 12, "starts at column {min_x}");
        }
    }
}
