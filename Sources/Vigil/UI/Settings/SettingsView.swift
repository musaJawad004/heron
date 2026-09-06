import SwiftUI

/// Preferences window.
struct SettingsView: View {
    @Environment(AppState.self) private var state

    var body: some View {
        // STUB — the menubar agent replaces this.
        @Bindable var settings = state.settings
        Form {
            Toggle("Show floating panel", isOn: $settings.showPanel)
            Toggle("Notify when Claude needs permission", isOn: $settings.notifyOnPermission)
            Toggle("Notify when Claude finishes", isOn: $settings.notifyOnDone)
        }
        .formStyle(.grouped)
        .frame(width: 420)
        .padding()
    }
}
