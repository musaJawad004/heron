---
name: heron-build
description: Build, bundle, run, test and reinstall Heron.app from the Swift package (no Xcode project, ad-hoc signed, fully local). Use for "build the app", "run it", "reinstall", "why won't it launch", or before verifying a UI change on the real Mac.
---
# Building and running Heron

Heron is a plain Swift package. `Scripts/build-app.sh` turns the executable into
`dist/Heron.app` with `Resources/Info.plist`, the SwiftPM resource bundle and an
**ad-hoc** signature (`codesign -s -`). No Apple developer identity, no
notarization, nothing is uploaded anywhere.

| Task | Command |
|---|---|
| Compile only | `swift build` |
| Unit tests | `swift test` |
| Build the .app | `make app` (→ `dist/Heron.app`) |
| Run it | `pkill -x Heron; make run` |
| Install to /Applications | `make install` |
| Format | `swift format --in-place --configuration .swift-format <file>` |
| Logs | `log stream --predicate 'subsystem == "com.glixentech.heron"' --style compact` |

Gotchas
- `UNUserNotificationCenter` crashes when the binary runs outside a `.app`
  bundle. Always test notifications through `dist/Heron.app`, never via
  `swift run`.
- Because the app is ad-hoc signed, macOS ties TCC grants (notifications,
  Apple Events) to the bundle path + signature. Rebuilding keeps them as long
  as the bundle id and path are unchanged; moving the app resets them.
- Login item (`SMAppService`) only works from `/Applications` or
  `~/Applications`.
- `LSUIElement` is true: the app has no Dock icon. Quit it with ⌘Q from its
  menu, or `pkill -x Heron`.
- If a stale copy is running, `open dist/Heron.app` silently focuses it;
  `pkill -x Heron` first.
