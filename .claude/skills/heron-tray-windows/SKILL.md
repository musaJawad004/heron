---
name: heron-tray-windows
description: The exact Tauri v2 recipe Heron uses for a tray-only app with a popover under the tray icon, a non-activating always-on-top floating panel, transparent windows with OS materials, and a hide-on-close settings window — including per-OS positioning, activation policy and the pitfalls hit on macOS, Windows and Linux. Use when touching src-tauri/src/tray.rs, windows.rs, lib.rs or tauri.conf.json windows.
---
# Tray, popover and panel in Heron

Cargo features: `tauri = { features = ["macos-private-api", "tray-icon", "image-png"] }`,
`tauri-plugin-positioner = { features = ["tray-icon"] }`. `macOSPrivateApi: true`
in `tauri.conf.json` is required for transparent windows on macOS.

## Tray
```rust
TrayIconBuilder::with_id("main")
    .icon(Image::from_bytes(include_bytes!("../icons/tray/tray-black.png"))?)
    .icon_as_template(true)          // macOS: auto light/dark; ignored elsewhere
    .show_menu_on_left_click(false)  // left = toggle popover, right = menu
    .on_tray_icon_event(|tray, event| {
        tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event); // REQUIRED for TrayCenter
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            windows::toggle_popover(tray.app_handle());
        }
    })
```
`tray.set_title(Some("3"))` shows text next to the icon on macOS only. On
Windows/Linux draw the count into the icon (`image` crate) or rely on tooltip.
The tray-icon crate scales the NSImage to 18 pt high: ship a 64×64 PNG.

## Popover (label `popover`)
`decorations:false, transparent:true, alwaysOnTop:true, skipTaskbar:true,
visible:false, windowEffects:{effects:["popover"]}`. Show = `move_window(Position::TrayCenter)`
(macOS, tray at top) / `TrayBottomCenter` (Windows, tray at bottom) then
`show()` + `set_focus()`. Hide on `WindowEvent::Focused(false)` (lib.rs).
On macOS also `app.hide()` if you want it gone from Mission Control.

## Panel (label `panel`)
`focusable:false` + `alwaysOnTop` + `visibleOnAllWorkspaces` + `skipTaskbar` +
`windowEffects:["hudWindow"]`. Drag with `data-tauri-drag-region` on the root
element (needs `core:window:allow-start-dragging`). Content-sized: the page
measures itself with `ResizeObserver` and calls `panel_resized(w,h)`; Rust
resizes the window and keeps the anchored edge fixed. Placement math lives in
`windows::place_panel` and must use `Monitor::work_area()` when available so the
menu bar / taskbar is respected. Persist the origin after `WindowEvent::Moved`
debounced ~300 ms.

## Settings (label `settings`)
Normal window, `visible:false`; `CloseRequested` → `hide()` + `prevent_close()`.
On macOS call `app.show()` before `set_focus()` or an Accessory-policy app
will not come to the front.

## Activation policy
`app.set_activation_policy(ActivationPolicy::Accessory)` in `setup` hides the
Dock icon (plus `LSUIElement` in `src-tauri/Info.plist`). Dialogs (folder
picker) still work but may open behind other apps: hide the popover first.

## Keep the process alive
`RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit()` —
otherwise hiding the last window quits the app.
