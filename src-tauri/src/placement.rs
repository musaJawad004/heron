//! Pure placement math for the floating panel and the popover. No Tauri
//! types, so every rule is unit-testable.
//!
//! Coordinates are **physical** pixels in the global desktop space (what
//! Tauri's `Monitor` and `outer_position` report). Window sizes come in as
//! **logical** pixels and are scaled per target monitor, because the same
//! window has a different physical size on a Retina and a 1× display.
//! Settings persist physical pixels plus the monitor name (`PanelOrigin`).

use crate::settings::{PanelEdge, PanelOrigin};

/// Inset from the work-area edge the panel is pinned at (logical px).
pub const MARGIN: f64 = 8.0;
/// A drag released within this distance of a left/right work-area edge snaps
/// to that edge (logical px).
pub const SNAP_DISTANCE: f64 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn max_x(&self) -> i32 {
        self.x + self.width as i32
    }

    pub fn max_y(&self) -> i32 {
        self.y + self.height as i32
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.max_x() && y >= self.y && y < self.max_y()
    }

    /// Shrink by `dx` on the left/right and `dy` on the top/bottom.
    pub fn inset(&self, dx: i32, dy: i32) -> Rect {
        let width = (self.width as i32 - 2 * dx).max(0) as u32;
        let height = (self.height as i32 - 2 * dy).max(0) as u32;
        Rect::new(self.x + dx, self.y + dy, width, height)
    }
}

/// What placement needs to know about one display.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorInfo {
    pub name: Option<String>,
    /// Full bounds; used to decide which monitor the cursor or a window is on.
    pub bounds: Rect,
    /// Bounds minus menu bar / taskbar / Dock.
    pub work_area: Rect,
    /// Scale of the space the rectangles above are in. `windows` normalises
    /// everything to points, so in production this is 1.0.
    pub scale: f64,
    /// The display's real backing scale factor. Only used to interpret raw
    /// platform rectangles (the tray icon), never by the placement math.
    pub device_scale: f64,
    pub is_primary: bool,
    pub has_cursor: bool,
}

/// A resolved panel position (physical px) and the monitor it lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub monitor: Option<String>,
}

/// Result of snapping a dragged panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapped {
    pub x: i32,
    pub y: i32,
    pub edge: PanelEdge,
}

fn px(logical: f64, scale: f64) -> i32 {
    (logical * scale).round() as i32
}

/// Keep a `width`×`height` (physical) window inside `area`. A window larger
/// than the area is pinned to its top/left corner.
pub fn clamp(x: i32, y: i32, width: u32, height: u32, area: &Rect) -> (i32, i32) {
    let max_x = (area.max_x() - width as i32).max(area.x);
    let max_y = (area.max_y() - height as i32).max(area.y);
    (x.clamp(area.x, max_x), y.clamp(area.y, max_y))
}

/// Which display's menu bar a tray rectangle belongs to.
///
/// macOS draws a menu bar on every display and reports the icon's rectangle in
/// that display's own pixels. Displays with different scale factors overlap in
/// that raw space (a 2x 1512-point built-in covers 0..3024, a 1x external
/// beside it covers 1512..3432), so the only way back is to try each display:
/// the right one is where dividing by its own scale lands inside its own
/// bounds. Menu-bar icons sit at the right end of the bar, so a secondary
/// display's icon never fits the primary; when the raw numbers do fit more than
/// one display the primary is the answer.
pub fn tray_monitor(tray_x: f64, tray_y: f64, monitors: &[MonitorInfo]) -> Option<usize> {
    let fits: Vec<usize> = monitors
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            let s = if m.device_scale > 0.0 { m.device_scale } else { 1.0 };
            m.bounds.contains((tray_x / s).round() as i32, (tray_y / s).round() as i32)
        })
        .map(|(i, _)| i)
        .collect();
    match fits.len() {
        0 => monitors.iter().position(|m| m.is_primary).or(if monitors.is_empty() { None } else { Some(0) }),
        1 => Some(fits[0]),
        _ => fits.iter().copied().find(|&i| monitors[i].is_primary).or(Some(fits[0])),
    }
}

/// The monitor placements default to: under the cursor, else the primary,
/// else the first one listed.
pub fn active_monitor(monitors: &[MonitorInfo]) -> Option<usize> {
    monitors
        .iter()
        .position(|m| m.has_cursor)
        .or_else(|| monitors.iter().position(|m| m.is_primary))
        .or(if monitors.is_empty() { None } else { Some(0) })
}

/// The monitor a window rectangle belongs to: the one under its centre, else
/// the one under its top-left corner.
pub fn monitor_for_window(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Option<usize> {
    let (cx, cy) = (x + width as i32 / 2, y + height as i32 / 2);
    monitors
        .iter()
        .position(|m| m.bounds.contains(cx, cy))
        .or_else(|| monitors.iter().position(|m| m.bounds.contains(x, y)))
}

/// Pin the panel to `edge` of `monitor`, `MARGIN` in from the work-area edge
/// and `MARGIN` below its top.
pub fn pinned(edge: PanelEdge, size: (f64, f64), monitor: &MonitorInfo) -> Placement {
    let wa = &monitor.work_area;
    let (w, h) = (px(size.0, monitor.scale), px(size.1, monitor.scale));
    let margin = px(MARGIN, monitor.scale);
    let x = match edge {
        PanelEdge::Left => wa.x + margin,
        PanelEdge::Right => wa.max_x() - w - margin,
    };
    let (x, y) = clamp(x, wa.y + margin, w as u32, h as u32, wa);
    Placement { x, y, monitor: monitor.name.clone() }
}

/// Where the panel goes.
///
/// - `saved` inside some monitor's work area → that position (clamped so the
///   whole panel stays inside). With `follow`, the offset from the anchored
///   edge is carried over to the monitor under the cursor instead.
/// - Otherwise pinned to `edge` of the active monitor (cursor → primary → first).
///
/// `size` is the panel's logical size. Returns `None` only without monitors.
pub fn placement(
    saved: Option<&PanelOrigin>,
    edge: PanelEdge,
    size: (f64, f64),
    monitors: &[MonitorInfo],
    follow: bool,
) -> Option<Placement> {
    let active = active_monitor(monitors)?;
    if let Some(o) = saved {
        // Prefer the monitor the origin was saved on when it still contains the
        // point; work areas can overlap in unusual arrangements.
        let on = monitors
            .iter()
            .position(|m| m.name.is_some() && m.name == o.monitor && m.work_area.contains(o.x, o.y))
            .or_else(|| monitors.iter().position(|m| m.work_area.contains(o.x, o.y)));
        if let Some(i) = on {
            // Only follow a cursor we can actually see; a failed cursor query
            // must not drag the panel back to the primary monitor.
            let target = if follow && monitors[active].has_cursor { active } else { i };
            return Some(if target == i {
                keep(o, size, &monitors[i])
            } else {
                translate(o, edge, size, &monitors[i], &monitors[target])
            });
        }
    }
    Some(pinned(edge, size, &monitors[active]))
}

fn keep(o: &PanelOrigin, size: (f64, f64), monitor: &MonitorInfo) -> Placement {
    let (w, h) = (px(size.0, monitor.scale), px(size.1, monitor.scale));
    let (x, y) = clamp(o.x, o.y, w as u32, h as u32, &monitor.work_area);
    Placement { x, y, monitor: monitor.name.clone() }
}

/// Carry a saved origin from `from` to `to`, keeping the logical offset from
/// the anchored edge and the top, so the panel lands in "the same place" on a
/// monitor of another size or scale.
fn translate(
    o: &PanelOrigin,
    edge: PanelEdge,
    size: (f64, f64),
    from: &MonitorInfo,
    to: &MonitorInfo,
) -> Placement {
    let (fa, ta) = (&from.work_area, &to.work_area);
    let w_from = px(size.0, from.scale);
    let (w_to, h_to) = (px(size.0, to.scale), px(size.1, to.scale));
    let dx = match edge {
        PanelEdge::Left => (o.x - fa.x) as f64 / from.scale,
        PanelEdge::Right => (fa.max_x() - (o.x + w_from)) as f64 / from.scale,
    };
    let dy = (o.y - fa.y) as f64 / from.scale;
    let x = match edge {
        PanelEdge::Left => ta.x + px(dx, to.scale),
        PanelEdge::Right => ta.max_x() - w_to - px(dx, to.scale),
    };
    let y = ta.y + px(dy, to.scale);
    let (x, y) = clamp(x, y, w_to as u32, h_to as u32, ta);
    Placement { x, y, monitor: to.name.clone() }
}

/// After a drag: snap to the left/right work-area edge when released within
/// `SNAP_DISTANCE` of it (nearest wins), keep the panel inside the work area,
/// and report which edge it is now anchored to.
pub fn snap(x: i32, y: i32, size: (f64, f64), monitor: &MonitorInfo, edge: PanelEdge) -> Snapped {
    let wa = &monitor.work_area;
    let (w, h) = (px(size.0, monitor.scale), px(size.1, monitor.scale));
    let margin = px(MARGIN, monitor.scale);
    let threshold = px(SNAP_DISTANCE, monitor.scale);
    let left_gap = x - wa.x;
    let right_gap = wa.max_x() - (x + w);
    let (x, edge) = if left_gap <= threshold && left_gap <= right_gap {
        (wa.x + margin, PanelEdge::Left)
    } else if right_gap <= threshold {
        (wa.max_x() - w - margin, PanelEdge::Right)
    } else {
        (x, edge)
    };
    let (x, y) = clamp(x, y, w as u32, h as u32, wa);
    Snapped { x, y, edge }
}

/// Where the popover goes: horizontally centred on the tray icon, just below
/// it when the bar is at the top of the screen (macOS, a top taskbar) and just
/// above it when the bar is at the bottom (Windows), kept inside the work
/// area with `gap` (physical px, horizontal/vertical) to spare.
pub fn popover_position(tray: &Rect, size: (u32, u32), work_area: &Rect, gap: (i32, i32)) -> (i32, i32) {
    let area = work_area.inset(gap.0, gap.1);
    let x = tray.x + tray.width as i32 / 2 - size.0 as i32 / 2;
    let y = if tray.max_y() <= work_area.y {
        area.y // bar above the work area: hang below it
    } else if tray.y >= work_area.max_y() {
        area.max_y() - size.1 as i32 // bar below the work area: sit above it
    } else {
        tray.max_y() + gap.1 // bar inside the work area (some Linux panels)
    };
    clamp(x, y, size.0, size.1, &area)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(name: &str, x: i32, y: i32, w: u32, h: u32, top_inset: u32, scale: f64) -> MonitorInfo {
        MonitorInfo {
            name: Some(name.into()),
            bounds: Rect::new(x, y, w, h),
            work_area: Rect::new(x, y + top_inset as i32, w, h - top_inset),
            scale,
            device_scale: scale,
            is_primary: false,
            has_cursor: false,
        }
    }

    /// Retina laptop (2×, 50 px menu bar) left of a 1× external monitor.
    fn two_monitors() -> Vec<MonitorInfo> {
        let mut a = mon("Built-in", 0, 0, 2880, 1800, 50, 2.0);
        a.is_primary = true;
        let b = mon("External", 2880, 0, 1920, 1080, 25, 1.0);
        vec![a, b]
    }

    fn origin(x: i32, y: i32, monitor: &str) -> PanelOrigin {
        PanelOrigin { x, y, monitor: Some(monitor.into()) }
    }

    const SIZE: (f64, f64) = (200.0, 44.0);

    #[test]
    fn no_monitors_no_placement() {
        assert_eq!(placement(None, PanelEdge::Left, SIZE, &[], true), None);
    }

    #[test]
    fn pins_left_with_margin_on_retina() {
        let m = two_monitors();
        let p = placement(None, PanelEdge::Left, SIZE, &m, false).unwrap();
        assert_eq!(p, Placement { x: 16, y: 50 + 16, monitor: Some("Built-in".into()) });
    }

    #[test]
    fn pins_right_using_physical_width() {
        let m = two_monitors();
        let p = placement(None, PanelEdge::Right, SIZE, &m, false).unwrap();
        assert_eq!(p.x, 2880 - 400 - 16);
        assert_eq!(p.y, 66);
    }

    #[test]
    fn pins_to_cursor_monitor_then_primary_then_first() {
        let mut m = two_monitors();
        m[1].has_cursor = true;
        let p = placement(None, PanelEdge::Left, SIZE, &m, false).unwrap();
        assert_eq!(p, Placement { x: 2888, y: 33, monitor: Some("External".into()) });

        m[1].has_cursor = false;
        assert_eq!(active_monitor(&m), Some(0), "primary wins without a cursor");
        m[0].is_primary = false;
        assert_eq!(active_monitor(&m), Some(0), "first listed as a last resort");
    }

    #[test]
    fn saved_origin_is_used_when_still_on_screen() {
        let mut m = two_monitors();
        m[0].has_cursor = true; // cursor elsewhere must not matter without follow
        let o = origin(3000, 400, "External");
        let p = placement(Some(&o), PanelEdge::Left, SIZE, &m, false).unwrap();
        assert_eq!(p, Placement { x: 3000, y: 400, monitor: Some("External".into()) });
    }

    #[test]
    fn saved_origin_is_clamped_inside_the_work_area() {
        let m = two_monitors();
        let o = origin(4790, 1070, "External"); // bottom-right corner, would overflow
        let p = placement(Some(&o), PanelEdge::Left, SIZE, &m, false).unwrap();
        assert_eq!((p.x, p.y), (4800 - 200, 1080 - 44));
    }

    #[test]
    fn stale_origin_falls_back_to_pinning() {
        let m = two_monitors();
        let o = origin(6000, 100, "Gone"); // monitor unplugged
        let p = placement(Some(&o), PanelEdge::Right, SIZE, &m, true).unwrap();
        assert_eq!(p, pinned(PanelEdge::Right, SIZE, &m[0]));
    }

    #[test]
    fn origin_in_the_menu_bar_counts_as_off_the_work_area() {
        let m = two_monitors();
        let o = origin(100, 10, "Built-in"); // inside bounds, above the work area
        let p = placement(Some(&o), PanelEdge::Left, SIZE, &m, false).unwrap();
        assert_eq!(p, pinned(PanelEdge::Left, SIZE, &m[0]));
    }

    #[test]
    fn follow_keeps_the_position_while_the_cursor_stays() {
        let mut m = two_monitors();
        m[1].has_cursor = true;
        let o = origin(3000, 400, "External");
        let p = placement(Some(&o), PanelEdge::Left, SIZE, &m, true).unwrap();
        assert_eq!((p.x, p.y), (3000, 400));
    }

    #[test]
    fn follow_translates_left_offset_across_scales() {
        let mut m = two_monitors();
        m[0].has_cursor = true;
        // 120 logical px in from the left, 375 logical px below the work-area top of the 1× monitor.
        let o = origin(2880 + 120, 25 + 375, "External");
        let p = placement(Some(&o), PanelEdge::Left, SIZE, &m, true).unwrap();
        assert_eq!(p, Placement { x: 240, y: 50 + 750, monitor: Some("Built-in".into()) });
    }

    #[test]
    fn follow_translates_right_offset_and_clamps() {
        let mut m = two_monitors();
        m[0].has_cursor = true;
        // On the 1× monitor: right edge 8 px from the work-area edge, 1000 px below its top.
        let o = origin(4800 - 200 - 8, 25 + 1000, "External");
        let p = placement(Some(&o), PanelEdge::Right, SIZE, &m, true).unwrap();
        assert_eq!(p.x, 2880 - 400 - 16, "8 logical px from the right edge, at 2×");
        // 1000 logical px below the top does not fit the 875-logical-px-tall Retina work area.
        assert_eq!(p.y, 1800 - 88);
        assert_eq!(p.monitor.as_deref(), Some("Built-in"));
    }

    #[test]
    fn prefers_the_named_monitor_when_work_areas_overlap() {
        let mut m = two_monitors();
        m[1].bounds = Rect::new(0, 0, 1920, 1080); // mirrored-ish layout
        m[1].work_area = Rect::new(0, 25, 1920, 1055);
        let o = origin(300, 300, "External");
        let p = placement(Some(&o), PanelEdge::Left, SIZE, &m, false).unwrap();
        assert_eq!(p.monitor.as_deref(), Some("External"));
    }

    #[test]
    fn snap_left_within_threshold() {
        let m = two_monitors();
        let s = snap(30, 200, SIZE, &m[0], PanelEdge::Right); // 30 px < 48 px threshold at 2×
        assert_eq!(s, Snapped { x: 16, y: 200, edge: PanelEdge::Left });
    }

    #[test]
    fn snap_right_within_threshold() {
        let m = two_monitors();
        let s = snap(2880 - 400 - 40, 300, SIZE, &m[0], PanelEdge::Left);
        assert_eq!(s, Snapped { x: 2880 - 400 - 16, y: 300, edge: PanelEdge::Right });
    }

    #[test]
    fn no_snap_in_the_middle_keeps_edge_and_clamps_y() {
        let m = two_monitors();
        let s = snap(1000, 10, SIZE, &m[0], PanelEdge::Right);
        assert_eq!(s, Snapped { x: 1000, y: 50, edge: PanelEdge::Right });
    }

    #[test]
    fn snap_prefers_the_nearer_edge_on_a_narrow_screen() {
        let narrow = mon("Tiny", 0, 0, 240, 400, 0, 1.0);
        let s = snap(30, 0, SIZE, &narrow, PanelEdge::Left); // left gap 30, right gap 10
        assert_eq!(s.edge, PanelEdge::Right);
        assert_eq!(s.x, 240 - 200 - 8);
    }

    #[test]
    fn clamp_pins_oversized_windows_to_the_origin() {
        let area = Rect::new(100, 50, 300, 200);
        assert_eq!(clamp(0, 0, 500, 500, &area), (100, 50));
        assert_eq!(clamp(350, 200, 100, 100, &area), (300, 150));
    }

    /// A 2x built-in beside a 1x external, normalised to points the way
    /// `windows::monitor_infos` does. In raw device pixels these two overlap
    /// (0..3024 and 1512..3432), which used to drop the panel half on each
    /// screen; in points they tile cleanly.
    fn mixed_dpi() -> Vec<MonitorInfo> {
        // Rectangles are points (what `windows::monitor_infos` produces);
        // `device_scale` stays the display's real backing factor.
        let mut built_in = mon("Color LCD", 0, 0, 1512, 982, 37, 1.0); // 3024x1964 @2x
        built_in.is_primary = true;
        built_in.device_scale = 2.0;
        let mut external = mon("2260W", 1512, 0, 1920, 1080, 25, 1.0); // 1920x1080 @1x
        external.device_scale = 1.0;
        vec![built_in, external]
    }

    #[test]
    fn tray_on_the_built_in_menu_bar_resolves_to_the_built_in() {
        let m = mixed_dpi();
        // Right end of the 2x built-in's menu bar: 1476 points -> 2952 pixels.
        // Those raw numbers also fit the external's 1512..3432, so the tie has
        // to break towards the primary.
        assert_eq!(tray_monitor(2952.0, 12.0, &m), Some(0));
    }

    #[test]
    fn tray_on_the_external_menu_bar_resolves_to_the_external() {
        let m = mixed_dpi();
        // Right end of the 1x external's bar: 3300 points and pixels alike.
        // Halving it lands at 1650, outside the built-in, so there is no tie.
        assert_eq!(tray_monitor(3300.0, 8.0, &m), Some(1));
    }

    #[test]
    fn tray_off_every_display_falls_back_to_the_primary() {
        let m = mixed_dpi();
        assert_eq!(tray_monitor(99_000.0, 99_000.0, &m), Some(0));
        assert_eq!(tray_monitor(0.0, 0.0, &[]), None);
    }

    #[test]
    fn mixed_dpi_monitors_do_not_overlap_in_points() {
        let m = mixed_dpi();
        assert_eq!(m[0].work_area.max_x(), m[1].work_area.x, "they tile, edge to edge");
        assert!(!m[0].bounds.contains(m[1].bounds.x, m[1].bounds.y), "no overlap");
    }

    #[test]
    fn panel_pinned_on_the_external_stays_on_the_external() {
        let m = mixed_dpi();
        for edge in [PanelEdge::Left, PanelEdge::Right] {
            let p = pinned(edge, SIZE, &m[1]);
            let wa = &m[1].work_area;
            assert!(p.x >= wa.x, "{edge:?}: x {} >= {}", p.x, wa.x);
            assert!(p.x + SIZE.0 as i32 <= wa.max_x(), "{edge:?}: right edge inside the external");
            assert_eq!(p.monitor.as_deref(), Some("2260W"));
        }
    }

    #[test]
    fn following_the_cursor_to_the_external_lands_fully_on_it() {
        let mut m = mixed_dpi();
        m[0].has_cursor = false;
        m[1].has_cursor = true;
        // Saved on the built-in, 8 px in from its left edge.
        let saved = origin(8, 45, "Color LCD");
        let p = placement(Some(&saved), PanelEdge::Left, SIZE, &m, true).expect("a placement");
        let wa = &m[1].work_area;
        assert_eq!(p.monitor.as_deref(), Some("2260W"), "moved to the cursor's screen");
        assert!(p.x >= wa.x && p.x + SIZE.0 as i32 <= wa.max_x(), "x {} inside the external", p.x);
        assert!(p.y >= wa.y && p.y + SIZE.1 as i32 <= wa.max_y(), "y {} inside the external", p.y);
    }

    #[test]
    fn a_grown_panel_stays_whole_on_its_own_monitor() {
        // The panel sits near the right edge of the external monitor and the
        // page then reports a wider, taller size (collapsed -> expanded).
        // `panel_resized` picks the monitor from the pre-resize rect and clamps
        // the new rect into it, so the panel must not spill onto the built-in.
        let m = two_monitors();
        let (before_w, before_h) = (76, 35);
        let (after_w, after_h) = (220, 180);
        let (x, y) = (2880 + 1920 - 84, 100); // 8 px from the external's right edge
        let i = monitor_for_window(&m, x, y, before_w, before_h).expect("on the external");
        assert_eq!(i, 1);
        let (nx, ny) = clamp(x, y, after_w, after_h, &m[i].work_area);
        let wa = &m[i].work_area;
        assert!(nx >= wa.x && nx + after_w as i32 <= wa.max_x(), "x {nx} inside {}..{}", wa.x, wa.max_x());
        assert!(ny >= wa.y && ny + after_h as i32 <= wa.max_y(), "y {ny} inside {}..{}", wa.y, wa.max_y());
        assert_eq!(nx, wa.max_x() - after_w as i32, "pulled back onto the external monitor");
    }

    #[test]
    fn monitor_for_window_uses_centre_then_corner() {
        let m = two_monitors();
        assert_eq!(monitor_for_window(&m, 2800, 100, 200, 44), Some(1), "centre on the external");
        assert_eq!(monitor_for_window(&m, 2700, 100, 200, 44), Some(0));
        assert_eq!(monitor_for_window(&m, 9000, 9000, 200, 44), None);
    }

    #[test]
    fn popover_hangs_below_a_top_menu_bar_centred_on_the_icon() {
        let work_area = Rect::new(0, 50, 2880, 1750); // Retina, 25 pt menu bar
        let tray = Rect::new(2000, 0, 60, 50);
        let (x, y) = popover_position(&tray, (680, 920), &work_area, (16, 8));
        // Centred under the icon; far enough from the edge that no clamping applies.
        assert_eq!((x, y), (2000 + 30 - 340, 58));
    }

    #[test]
    fn popover_centre_gives_way_to_the_inset_work_area() {
        // An icon this close to the corner would push the window past the gap,
        // so the clamp wins over centring.
        let work_area = Rect::new(0, 50, 2880, 1750);
        let tray = Rect::new(2500, 0, 60, 50);
        let (x, y) = popover_position(&tray, (680, 920), &work_area, (16, 8));
        assert_eq!((x, y), (2880 - 16 - 680, 58));
    }

    #[test]
    fn popover_opens_on_the_screen_whose_menu_bar_was_clicked() {
        // Tray icon near the right of the external monitor's own menu bar,
        // already converted to points with that monitor's scale.
        let m = mixed_dpi();
        let external = &m[1];
        let tray = Rect::new(3300, 0, 24, 25);
        let (x, y) = popover_position(&tray, (340, 447), &external.work_area, (8, 4));
        let wa = &external.work_area;
        assert!(x >= wa.x, "x {x} is not back on the built-in");
        assert!(x + 340 <= wa.max_x(), "x {x} stays inside the external");
        assert!(y >= wa.y && y + 447 <= wa.max_y(), "y {y} inside the external");
    }

    #[test]
    fn popover_never_overflows_the_right_edge() {
        let work_area = Rect::new(0, 25, 1920, 1055);
        let tray = Rect::new(1880, 0, 30, 25); // icon near the corner
        let (x, _) = popover_position(&tray, (340, 460), &work_area, (8, 4));
        assert_eq!(x, 1920 - 8 - 340);
    }

    #[test]
    fn popover_sits_above_a_bottom_taskbar() {
        let work_area = Rect::new(0, 0, 1920, 1040);
        let tray = Rect::new(1700, 1040, 24, 40);
        let (x, y) = popover_position(&tray, (340, 460), &work_area, (8, 4));
        assert_eq!((x, y), (1700 + 12 - 170, 1040 - 4 - 460));
    }

    #[test]
    fn popover_below_a_panel_inside_the_work_area() {
        let work_area = Rect::new(0, 0, 1920, 1080); // panel not excluded from the work area
        let tray = Rect::new(1700, 0, 24, 28);
        let (_, y) = popover_position(&tray, (340, 460), &work_area, (8, 4));
        assert_eq!(y, 28 + 4);
    }

    #[test]
    fn rect_inset() {
        assert_eq!(Rect::new(0, 0, 100, 50).inset(8, 4), Rect::new(8, 4, 84, 42));
        assert_eq!(Rect::new(0, 0, 10, 10).inset(8, 8).width, 0);
    }
}
