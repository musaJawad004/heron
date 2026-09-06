import AppKit
import SwiftUI

/// Hosts `PanelView` in a borderless, floating, non-activating `NSPanel`
/// that sits at the edge of the active screen and can be dragged anywhere.
///
/// Contract (implemented by the panel agent):
/// - `.floating` level, `.canJoinAllSpaces` + `.fullScreenAuxiliary`,
///   `isMovableByWindowBackground`, transparent background with the SwiftUI
///   view drawing its own material; `hidesOnDeactivate = false`.
/// - `setVisible(true)` positions the panel: if `settings.panelOrigin` is set
///   and still on some screen, use it; otherwise pin to `settings.panelEdge`
///   of the screen under the mouse (or main screen), vertically centred.
/// - When `settings.panelFollowsActiveScreen` is true, re-position on
///   `NSApplication.didChangeScreenParametersNotification` and when the active
///   screen changes (poll mouse screen every ~2 s while visible).
/// - Persist the origin into `settings.panelOrigin` after a drag ends.
/// - Resize to fit content when `state.running` changes.
@MainActor
public final class FloatingPanelController {
    let state: AppState
    private var panel: NSPanel?

    public init(state: AppState) {
        self.state = state
    }

    public func setVisible(_ visible: Bool) {
        // STUB
        if visible { show() } else { hide() }
    }

    public func toggle() {
        setVisible(panel?.isVisible != true)
    }

    private func show() {
        if panel == nil {
            let p = NSPanel(
                contentRect: NSRect(x: 0, y: 0, width: 220, height: 120),
                styleMask: [.nonactivatingPanel, .borderless],
                backing: .buffered,
                defer: false
            )
            p.level = .floating
            p.isOpaque = false
            p.backgroundColor = .clear
            p.hasShadow = true
            p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
            p.isMovableByWindowBackground = true
            p.hidesOnDeactivate = false
            p.contentView = NSHostingView(rootView: PanelView().environment(state))
            panel = p
        }
        panel?.orderFrontRegardless()
    }

    private func hide() {
        panel?.orderOut(nil)
    }
}
