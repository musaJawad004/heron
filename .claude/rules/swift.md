---
paths:
  - "Sources/**/*.swift"
  - "Tests/**/*.swift"
---
# Swift conventions

- Swift 6 toolchain, language mode 5 with `StrictConcurrency` warnings on.
  Treat every concurrency warning as an error to fix, not to silence.
- UI and `AppState` are `@MainActor`. Background work (file scans, transcript
  parsing) lives in actors (`TranscriptIndex`) or on `DispatchQueue`s inside
  `@unchecked Sendable` classes that own their queue. Hand results back with
  `Task { @MainActor in … }`.
- State uses the Observation framework (`@Observable`), not Combine or
  `ObservableObject`. Views read state through `@Environment(AppState.self)`.
- Value types for models. `Session`, `HookEvent` are `Hashable` + `Sendable`;
  keep them free of AppKit/SwiftUI imports.
- No force unwraps and no `try!` outside tests. Failures surface through
  `AppState.lastError` (user-visible) or `os.Logger` (developer-visible).
  Use `Logger(subsystem: "com.glixentech.heron", category: "<module>")`.
- Parse JSON with `JSONSerialization` into `[String: Any]` for the loose,
  version-drifting Claude Code files (fields come and go); use `Codable`
  only for data Heron itself writes.
- Timestamps in Claude Code files are Unix **milliseconds**; convert once at
  the boundary and use `Date` everywhere else.
- Never read a transcript file fully. Read at most the first 64 KB via
  `FileHandle` and stop at the first line that satisfies the query.
- File-system watching: `DispatchSource.makeFileSystemObjectSource` on a
  directory fd with `.write`, plus a coarse poll as backstop. Always close
  the fd in `cancelHandler`.
- Public API of each module is documented with `///` on the type and every
  public method, in the same voice as the existing stubs.
- Tests are XCTest under `Tests/HeronTests`, use fixture data written to a
  temp directory (never the real `~/.claude`), and run with `swift test`.
- Format with `swift format` using `.swift-format` (a hook does this on edit).
