import SwiftUI

/// Content of the menu bar popover window.
struct MenuBarView: View {
    @Environment(AppState.self) private var state

    var body: some View {
        // STUB — the menubar agent replaces this.
        VStack(alignment: .leading, spacing: 8) {
            Text("Heron").font(.headline)
            Text("\(state.runningCount) running").font(.caption)
            Button("New Session…") { state.newSessionWithPicker() }
            SettingsLink { Text("Settings…") }
            Button("Quit") { NSApp.terminate(nil) }
        }
        .padding()
        .frame(width: 320)
    }
}
