# Git & workflow

- Small, focused commits; message in imperative mood; body explains *why*.
- Work on a branch per feature/module; `main` stays buildable.
- Before committing: `swift build`, `swift test`, and `git diff --stat` review.
- Do not commit `dist/`, `.build/`, or anything from `~/Library`.
- Bump `CFBundleShortVersionString` in `Resources/Info.plist` and add a
  `CHANGELOG.md` entry for user-visible changes.
