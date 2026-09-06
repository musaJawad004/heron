import SwiftUI

/// The always-on-top "vigil" widget content.
struct PanelView: View {
    @Environment(AppState.self) private var state

    var body: some View {
        // STUB — the panel agent replaces this.
        VStack(alignment: .leading, spacing: 6) {
            Text("\(state.runningCount) Claude").font(.headline)
            ForEach(state.running) { s in
                HStack {
                    Circle().fill(Theme.color(for: s.status)).frame(width: 8, height: 8)
                    Text(s.projectName).lineLimit(1)
                }
            }
        }
        .padding(Theme.padding)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: Theme.cornerRadius))
    }
}
