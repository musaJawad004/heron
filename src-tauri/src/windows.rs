//! The three windows: `popover` (under the tray icon), `panel` (the floating
//! vigil at the screen edge) and `settings`.
//!
//! Contract:
//! - Popover: centred on the tray icon (rect captured from the tray click,
//!   see `note_tray_rect`) and clamped to the work area, hidden on blur
//!   (`popover_blurred`, wired in lib.rs), never in the Dock/taskbar. On
//!   macOS dismissing it hands focus back to the app the user came from.
//!   The page reports its height through `popover_resized` (120–560 px).
//! - Panel: non-activating, always on top (macOS: above full-screen apps),
//!   on all Spaces. Placed by `placement::placement`: the saved origin when
//!   it is still on a monitor, else pinned to `panel_edge` of the monitor
//!   under the cursor. Follows the active monitor when enabled (never within
//!   1 s of a user drag). After a drag (`WindowEvent::Moved`, debounced) it
//!   snaps to a near edge and persists `panel_origin` / `panel_edge`.
//! - Settings: a normal window; closing hides it. On macOS the app switches
//!   to the Regular activation policy while it is open so it can come to the
//!   front, and back to Accessory when it closes.
//!
//! Everything here works in **logical points**. macOS lays the desktop out
//! in points but reports each monitor's origin in that monitor's own pixels,
//! so on a mixed-DPI setup (a 2x built-in beside a 1x external) the physical
//! rectangles overlap and a window placed by physical coordinates lands on
//! the wrong screen, or half on each. `monitor_infos` divides every monitor
//! by its own scale to restore one non-overlapping space.
//!
//! All math lives in `placement` (pure, unit-tested); this file only reads
//! window/monitor state and applies results. Nothing here holds a lock while
//! calling into Tauri.

use crate::model::now_ms;
use crate::placement::{self, MonitorInfo, Rect};
use crate::settings::{PanelEdge, PanelOrigin};
use crate::state::AppState;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Monitor, Position, Size, WebviewWindow};

pub const POPOVER: &str = "popover";
pub const PANEL: &str = "panel";
pub const SETTINGS: &str = "settings";

const POPOVER_WIDTH: f64 = 340.0;
const POPOVER_MIN_HEIGHT: f64 = 120.0;
const POPOVER_MAX_HEIGHT: f64 = 560.0;
/// Space between the popover and the screen edge / menu bar (logical px).
const POPOVER_GAP: (f64, f64) = (8.0, 4.0);
/// Sanity bounds for the content-sized panel (logical px).
const PANEL_MIN: (f64, f64) = (48.0, 24.0);
const PANEL_MAX: (f64, f64) = (320.0, 600.0);

/// A blur this soon after `show` is the OS settling focus, not the user leaving.
const BLUR_GRACE_MS: u64 = 150;
/// A tray click this soon after a blur-hide is the click that caused the blur.
const REOPEN_GRACE_MS: u64 = 250;
/// Follow-active-screen waits this long after the user last moved the panel.
const DRAG_SETTLE_MS: u64 = 1000;
/// Snap + persist this long after the last `Moved` event.
const MOVE_DEBOUNCE_MS: u64 = 300;
const FOLLOW_INTERVAL_MS: u64 = 2000;
const TICK_MS: u64 = 100;

static POPOVER_SHOWN_AT: AtomicU64 = AtomicU64::new(0);
static POPOVER_HIDDEN_AT: AtomicU64 = AtomicU64::new(0);
static PANEL_MOVED_AT: AtomicU64 = AtomicU64::new(0);
static PANEL_MOVE_PENDING: AtomicBool = AtomicBool::new(false);
static PANEL_THREAD_STARTED: AtomicBool = AtomicBool::new(false);
/// Tray icon rect (physical px) from the last tray event.
/// The tray icon's rectangle as the platform reports it: device pixels of the
/// screen whose menu bar holds the icon.
#[derive(Clone, Copy, Debug)]
struct PhysicalRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

static TRAY_RECT: Mutex<Option<PhysicalRect>> = Mutex::new(None);

fn window(app: &AppHandle, label: &str) -> Option<WebviewWindow> {
    app.get_webview_window(label)
}

fn is_visible(app: &AppHandle, label: &str) -> bool {
    window(app, label).and_then(|w| w.is_visible().ok()).unwrap_or(false)
}

fn since(at: &AtomicU64) -> u64 {
    now_ms().saturating_sub(at.load(Ordering::Relaxed))
}

/// Move a window, in **logical points** (see `monitor_infos`).
fn move_to(w: &WebviewWindow, x: i32, y: i32) {
    if logical_position(w).map(|p| p == (x, y)).unwrap_or(false) {
        return;
    }
    let _ = w.set_position(Position::Logical(LogicalPosition::new(x as f64, y as f64)));
}

fn logical_size(w: &WebviewWindow) -> Option<(f64, f64)> {
    let scale = w.scale_factor().ok()?;
    let s = w.outer_size().ok()?.to_logical::<f64>(scale);
    Some((s.width, s.height))
}

/// A window's top-left in logical points.
fn logical_position(w: &WebviewWindow) -> Option<(i32, i32)> {
    let scale = w.scale_factor().ok()?;
    let p = w.outer_position().ok()?.to_logical::<f64>(scale);
    Some((p.x.round() as i32, p.y.round() as i32))
}

/// Divide a monitor rectangle by that monitor's own scale, giving points.
fn to_points(x: i32, y: i32, width: u32, height: u32, scale: f64) -> Rect {
    let s = if scale > 0.0 { scale } else { 1.0 };
    Rect::new(
        (x as f64 / s).round() as i32,
        (y as f64 / s).round() as i32,
        (width as f64 / s).round() as u32,
        (height as f64 / s).round() as u32,
    )
}

fn work_area_of(m: &Monitor) -> Rect {
    let wa = m.work_area();
    to_points(wa.position.x, wa.position.y, wa.size.width, wa.size.height, m.scale_factor())
}

fn bounds_of(m: &Monitor) -> Rect {
    to_points(m.position().x, m.position().y, m.size().width, m.size().height, m.scale_factor())
}

fn primary_monitor(app: &AppHandle) -> Option<Monitor> {
    app.primary_monitor().ok().flatten()
}

/// Snapshot of every monitor for the placement math.
fn monitor_infos(app: &AppHandle) -> Vec<MonitorInfo> {
    let primary = primary_monitor(app);
    // Ask the platform which monitor holds the cursor rather than testing our
    // own rectangles: the cursor arrives in a different space again.
    let under_cursor =
        app.cursor_position().ok().and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten());
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let same = |o: &Monitor| o.position() == m.position() && o.size() == m.size();
            MonitorInfo {
                name: m.name().cloned(),
                bounds: bounds_of(&m),
                work_area: work_area_of(&m),
                // Rectangles and window sizes are already in points, so the
                // placement math needs no further scaling.
                scale: 1.0,
                is_primary: primary.as_ref().map(same).unwrap_or(false),
                has_cursor: under_cursor.as_ref().map(same).unwrap_or(false),
            }
        })
        .collect()
}

// ---- popover ----------------------------------------------------------------

/// Remember where the tray icon is (called from every tray event).
pub fn note_tray_rect(rect: &tauri::Rect) {
    // Kept in the device pixels the platform reports, because the scale that
    // converts them belongs to whichever screen's menu bar was clicked, and
    // that is only known once we can look at the cursor (see `tray_monitor`).
    let p = rect.position.to_physical::<f64>(1.0);
    let s = rect.size.to_physical::<f64>(1.0);
    *TRAY_RECT.lock() =
        Some(PhysicalRect { x: p.x, y: p.y, width: s.width.max(1.0), height: s.height.max(1.0) });
}

pub fn toggle_popover(app: &AppHandle) {
    if is_visible(app, POPOVER) {
        hide_popover(app);
    } else if since(&POPOVER_HIDDEN_AT) < REOPEN_GRACE_MS {
        // The blur caused by this very click already closed it (always on
        // Windows, sometimes on macOS): reopening would make the click a no-op.
    } else {
        show_popover(app);
    }
}

pub fn show_popover(app: &AppHandle) {
    let Some(w) = window(app, POPOVER) else { return };
    // Undo a previous `NSApp.hide` (see `hide_popover`). Does not touch the Dock.
    #[cfg(target_os = "macos")]
    let _ = app.show();
    position_popover(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    // AppKit resets a window's level when it is ordered out and back in, so an
    // Accessory app's popover would reappear underneath ordinary app windows.
    // Re-assert the level and order it front every time it is shown.
    raise_above_apps(app, POPOVER);
    POPOVER_SHOWN_AT.store(now_ms(), Ordering::Relaxed);
}

/// Put a window back at the floating level and order it in front, without
/// activating the app. No-op off macOS, where `always_on_top` already holds.
fn raise_above_apps(app: &AppHandle, label: &'static str) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSStatusWindowLevel, NSWindow};

        let Some(ptr) = window(app, label).and_then(|w| w.ns_window().ok()).map(|p| p as usize) else {
            return;
        };
        let _ = app.run_on_main_thread(move || {
            // SAFETY: the pointer came from Tauri for a live NSWindow (windows
            // are only ever hidden, never destroyed) and AppKit is touched on
            // the main thread only.
            let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
            ns.setLevel(NSStatusWindowLevel - 1);
            ns.orderFrontRegardless();
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, label);
    }
}

/// Hide the popover. On macOS also hand focus back to the app the user came
/// from — unless Settings is open, since `NSApp.hide` would take it along.
/// The panel opts out of app hiding (`canHide = false`, see `configure_native`).
pub fn hide_popover(app: &AppHandle) {
    let Some(w) = window(app, POPOVER) else { return };
    let was_visible = w.is_visible().unwrap_or(false);
    let _ = w.hide();
    if was_visible {
        POPOVER_HIDDEN_AT.store(now_ms(), Ordering::Relaxed);
        #[cfg(target_os = "macos")]
        if !is_visible(app, SETTINGS) {
            let _ = app.hide();
        }
    }
}

/// `WindowEvent::Focused(false)` on the popover.
pub fn popover_blurred(app: &AppHandle) {
    if since(&POPOVER_SHOWN_AT) < BLUR_GRACE_MS {
        return; // macOS sometimes drops focus right after `show`
    }
    hide_popover(app);
}

/// The popover page reports its content height; resize (120–560 logical px)
/// and keep it attached to the tray icon.
pub fn popover_resized(app: &AppHandle, height: f64) {
    let Some(w) = window(app, POPOVER) else { return };
    let height = if height.is_finite() {
        height.clamp(POPOVER_MIN_HEIGHT, POPOVER_MAX_HEIGHT)
    } else {
        POPOVER_MIN_HEIGHT
    };
    if let Some((_, current)) = logical_size(&w) {
        if (current - height).abs() < 0.5 {
            return;
        }
    }
    let _ = w.set_size(Size::Logical(LogicalSize::new(POPOVER_WIDTH, height)));
    if w.is_visible().unwrap_or(false) {
        // macOS keeps the bottom-left corner on resize; re-anchor the top edge
        // under the tray (Windows' bottom taskbar case re-anchors the bottom).
        position_popover(app, &w);
    }
}

/// Centre the popover on the tray icon and keep it inside the work area. The
/// position is computed up front from the captured tray rect rather than read
/// back after a move, because moves and resizes are asynchronous on macOS.
fn position_popover(app: &AppHandle, w: &WebviewWindow) {
    let Some((width, height)) = logical_size(w) else { return };
    let size = (width.round() as u32, height.round() as u32);
    let tray = *TRAY_RECT.lock();
    let target = match tray.and_then(|t| tray_monitor(app, &t).map(|m| (t, m))) {
        Some((tray, m)) => {
            let scale = if m.scale_factor() > 0.0 { m.scale_factor() } else { 1.0 };
            let tray = Rect::new(
                (tray.x / scale).round() as i32,
                (tray.y / scale).round() as i32,
                (tray.width / scale).round() as u32,
                (tray.height / scale).round() as u32,
            );
            Some(placement::popover_position(&tray, size, &work_area_of(&m), gap_px(&m)))
        }
        // No tray rect yet (Linux app indicators never report one, and the
        // single-instance hook can arrive before any click): top-right corner.
        None => primary_monitor(app).map(|m| {
            let gap = gap_px(&m);
            let area = work_area_of(&m).inset(gap.0, gap.1);
            placement::clamp(area.max_x() - size.0 as i32, area.y, size.0, size.1, &area)
        }),
    };
    if let Some((x, y)) = target {
        move_to(w, x, y);
    }
}

/// Which screen's menu bar the tray icon was clicked on.
///
/// macOS shows a menu bar on every display, and the rectangle the tray hands us
/// is in that display's own pixels. Converting it with the wrong scale is what
/// used to throw the popover onto the primary screen, so take the screen under
/// the cursor: clicking the icon puts the pointer on it. Fall back to reading
/// the rectangle as primary-screen pixels.
fn tray_monitor(app: &AppHandle, tray: &PhysicalRect) -> Option<Monitor> {
    app.cursor_position()
        .ok()
        .and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten())
        .or_else(|| {
            let (cx, cy) = (tray.x + tray.width / 2.0, tray.y + tray.height / 2.0);
            app.monitor_from_point(cx, cy).ok().flatten()
        })
        .or_else(|| primary_monitor(app))
}

/// Popover gap in points (the whole module works in points).
fn gap_px(_m: &Monitor) -> (i32, i32) {
    (POPOVER_GAP.0.round() as i32, POPOVER_GAP.1.round() as i32)
}

// ---- settings ---------------------------------------------------------------

pub fn open_settings(app: &AppHandle) {
    hide_popover(app);
    let Some(w) = window(app, SETTINGS) else { return };
    #[cfg(target_os = "macos")]
    {
        // An Accessory app cannot come to the front; show a Dock icon while
        // Settings is open (the usual trick for menu bar apps).
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        let _ = app.show();
    }
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
}

/// `WindowEvent::CloseRequested` on settings (the close itself is prevented).
pub fn settings_close_requested(app: &AppHandle) {
    if let Some(w) = window(app, SETTINGS) {
        let _ = w.hide();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        let _ = app.hide(); // give focus back to the previous app
    }
}

// ---- panel ------------------------------------------------------------------

pub fn toggle_panel(app: &AppHandle) {
    let state = app.state::<AppState>();
    let visible = !state.settings().show_panel;
    let _ = state.update_settings(app, serde_json::json!({ "showPanel": visible }));
}

pub fn set_panel_visible(app: &AppHandle, visible: bool) {
    let Some(w) = window(app, PANEL) else { return };
    if visible {
        place_panel(app);
        let _ = w.show();
        configure_native(app); // cheap; guards against anything resetting the level
    } else {
        let _ = w.hide();
    }
}

/// Put the panel at its saved origin or pinned to the configured edge.
pub fn place_panel(app: &AppHandle) {
    let Some(w) = window(app, PANEL) else { return };
    let Some(size) = logical_size(&w) else { return };
    let settings = app.state::<AppState>().settings();
    let monitors = monitor_infos(app);
    let Some(p) = placement::placement(
        settings.panel_origin.as_ref(),
        settings.panel_edge,
        size,
        &monitors,
        settings.panel_follows_active_screen,
    ) else {
        return;
    };
    move_to(&w, p.x, p.y);
}

/// `WindowEvent::Moved` on the panel: remember when, and schedule
/// `panel_moved` once the events stop (see `start_panel_thread`).
pub fn panel_window_moved() {
    PANEL_MOVED_AT.store(now_ms(), Ordering::Relaxed);
    PANEL_MOVE_PENDING.store(true, Ordering::Relaxed);
}

/// After a drag: snap to a near edge and persist the origin. Also reached
/// through the `panel_moved` command. Idempotent, so the `Moved` event a snap
/// itself produces settles in one extra pass.
pub fn panel_moved(app: &AppHandle) {
    let Some(w) = window(app, PANEL) else { return };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    let (Some((px, py)), Some(size)) = (logical_position(&w), logical_size(&w)) else { return };
    let monitors = monitor_infos(app);
    let Some(i) =
        placement::monitor_for_window(&monitors, px, py, size.0.round() as u32, size.1.round() as u32)
            .or_else(|| placement::active_monitor(&monitors))
    else {
        return;
    };
    let state = app.state::<AppState>();
    let settings = state.settings();
    let s = placement::snap(px, py, size, &monitors[i], settings.panel_edge);
    move_to(&w, s.x, s.y);
    let origin = PanelOrigin { x: s.x, y: s.y, monitor: monitors[i].name.clone() };
    if settings.panel_origin.as_ref() == Some(&origin) && settings.panel_edge == s.edge {
        return;
    }
    // An edge change makes `update_settings` call `place_panel`, which lands
    // on this same origin: no loop.
    let _ = state.update_settings(app, serde_json::json!({ "panelOrigin": origin, "panelEdge": s.edge }));
}

/// Resize the panel to the content size reported by the page, keeping the
/// anchored edge where it is.
pub fn panel_resized(app: &AppHandle, width: f64, height: f64) {
    let Some(w) = window(app, PANEL) else { return };
    if !width.is_finite() || !height.is_finite() {
        return;
    }
    let width = width.clamp(PANEL_MIN.0, PANEL_MAX.0);
    let height = height.clamp(PANEL_MIN.1, PANEL_MAX.1);
    let (Some((px, py)), Some((old_w, old_h))) = (logical_position(&w), logical_size(&w)) else {
        return;
    };
    let (new_w, new_h) = (width.round() as i32, height.round() as i32);
    let edge = app.state::<AppState>().settings().panel_edge;
    let _ = w.set_size(Size::Logical(LogicalSize::new(width, height)));
    // macOS keeps the bottom-left corner on resize, so the top-left is
    // re-applied on every OS; a right-anchored panel also shifts by the delta.
    let x = match edge {
        PanelEdge::Left => px,
        PanelEdge::Right => px + old_w.round() as i32 - new_w,
    };
    // Growing the panel must not push it onto the neighbouring screen: keep it
    // whole on the monitor it was already on (chosen from the pre-resize rect,
    // so an expansion cannot hand it to the screen it is spilling towards).
    let monitors = monitor_infos(app);
    let (before_w, before_h) = (old_w.round() as u32, old_h.round() as u32);
    let (x, y) = match placement::monitor_for_window(&monitors, px, py, before_w, before_h) {
        Some(i) => placement::clamp(x, py, new_w as u32, new_h as u32, &monitors[i].work_area),
        None => (x, py),
    };
    move_to(&w, x, y);
}

/// One background thread for the panel: debounced snap/persist after moves
/// and, every 2 s, following the monitor under the cursor.
pub fn start_panel_thread(app: &AppHandle) {
    if PANEL_THREAD_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    let spawned = thread::Builder::new().name("heron-panel".into()).spawn(move || {
        let follow_every = FOLLOW_INTERVAL_MS / TICK_MS;
        let mut ticks: u64 = 0;
        loop {
            thread::sleep(Duration::from_millis(TICK_MS));
            ticks += 1;
            if PANEL_MOVE_PENDING.load(Ordering::Relaxed) && since(&PANEL_MOVED_AT) >= MOVE_DEBOUNCE_MS {
                PANEL_MOVE_PENDING.store(false, Ordering::Relaxed);
                panel_moved(&handle);
            }
            if ticks % follow_every == 0 {
                follow_active_screen(&handle);
            }
        }
    });
    if let Err(e) = spawned {
        log::warn!("panel thread failed to start: {e}");
    }
}

fn follow_active_screen(app: &AppHandle) {
    let settings = app.state::<AppState>().settings();
    if !settings.show_panel
        || !settings.panel_follows_active_screen
        || since(&PANEL_MOVED_AT) < DRAG_SETTLE_MS
    {
        return;
    }
    let Some(w) = window(app, PANEL) else { return };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else { return };
    let monitors = monitor_infos(app);
    let Some(active) = monitors.iter().position(|m| m.has_cursor) else { return };
    if placement::monitor_for_window(&monitors, pos.x, pos.y, size.width, size.height) == Some(active) {
        return;
    }
    place_panel(app);
}

/// Native window tweaks Tauri's config cannot express. macOS: put the panel
/// (and the popover, so it can still draw above the panel) at the level just
/// under status items, let both join full-screen Spaces, and keep the panel
/// out of `NSApp.hide` / Mission Control. No-op elsewhere.
pub fn configure_native(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};

        let targets: Vec<(usize, bool)> = [(PANEL, true), (POPOVER, false)]
            .into_iter()
            .filter_map(|(label, is_panel)| {
                let w = window(app, label)?;
                w.ns_window().ok().map(|p| (p as usize, is_panel))
            })
            .collect();
        let _ = app.run_on_main_thread(move || {
            for (ptr, is_panel) in targets {
                // SAFETY: the pointer came from Tauri for a live NSWindow that
                // is never destroyed (windows are only hidden), and AppKit is
                // touched on the main thread only.
                let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
                ns.setLevel(NSStatusWindowLevel - 1);
                let behaviour = if is_panel {
                    NSWindowCollectionBehavior::CanJoinAllSpaces
                        | NSWindowCollectionBehavior::FullScreenAuxiliary
                        | NSWindowCollectionBehavior::Stationary
                        | NSWindowCollectionBehavior::IgnoresCycle
                } else {
                    NSWindowCollectionBehavior::MoveToActiveSpace
                        | NSWindowCollectionBehavior::FullScreenAuxiliary
                        | NSWindowCollectionBehavior::Transient
                        | NSWindowCollectionBehavior::IgnoresCycle
                };
                ns.setCollectionBehavior(behaviour);
                ns.setHidesOnDeactivate(false);
                if is_panel {
                    ns.setCanHide(false);
                }
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }
}

// ---- actions ----------------------------------------------------------------

/// Open the folder picker, then start a session there.
pub fn new_session_with_picker(app: &AppHandle) {
    use tauri_plugin_dialog::DialogExt;
    hide_popover(app);
    // `hide_popover` may have hidden the app; the picker must be visible.
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let state = app.state::<AppState>();
    let base = state.settings().default_projects_folder;
    let mut dialog = app.dialog().file().set_title("Start Claude Code in…");
    if !base.is_empty() {
        dialog = dialog.set_directory(base);
    }
    let handle = app.clone();
    dialog.pick_folder(move |picked| {
        if let Some(path) = picked.and_then(|p| p.into_path().ok()) {
            let state = handle.state::<AppState>();
            state.new_session(&handle, &path);
        }
    });
}
