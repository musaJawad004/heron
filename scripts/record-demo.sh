#!/bin/bash
# Records the README demo GIF from the real app, using a throwaway Claude
# config so no real session ever appears in the recording.
#
#   scripts/record-demo.sh          (needs ffmpeg and cliclick, and an awake screen)
#
# Everything it creates lives under /tmp and is removed at the end, except
# docs/media/demo.gif.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v ffmpeg >/dev/null || { echo "brew install ffmpeg"; exit 1; }
command -v cliclick >/dev/null || { echo "brew install cliclick"; exit 1; }

DEMO=/tmp/heron-demo
APP=${APP:-/Applications/Heron.app/Contents/MacOS/heron}
rm -rf "$DEMO"; mkdir -p "$DEMO/sessions" "$DEMO/bin"
printf '#!/bin/sh\nexit 0\n' > "$DEMO/bin/claude"; chmod +x "$DEMO/bin/claude"

# Stand-in processes whose argv[0] is "claude", so the liveness check that
# guards against stale registry files accepts the fixture sessions.
python3 - "$DEMO" <<'PY'
import json, os, subprocess, sys, time
root, now = sys.argv[1], int(time.time() * 1000)
demo = [("acme-api-3f", "/Users/demo/code/acme-api", "busy",
         "Add rate limiting to the public endpoints", "feat/rate-limit"),
        ("storefront-9c", "/Users/demo/code/storefront", "idle",
         "Fix the cart total when a coupon is removed", "main"),
        ("infra-2a", "/Users/demo/code/infra", "busy",
         "Move the staging cluster to the new node pool", "main")]
for i, (name, cwd, status, title, branch) in enumerate(demo):
    pid = subprocess.Popen(["bash", "-c", "exec -a claude sleep 900"]).pid
    sid = f"{pid:08x}-1111-2222-3333-444455556666"
    json.dump({"pid": pid, "sessionId": sid, "cwd": cwd, "startedAt": now - 600_000,
               "version": "2.1.263", "kind": "interactive", "entrypoint": "cli",
               "pidDomain": "darwin", "name": name, "updatedAt": now - 5_000,
               "status": status, "statusUpdatedAt": now - 5_000},
              open(f"{root}/sessions/{pid}.json", "w"))
    proj = cwd.replace("/", "-")
    os.makedirs(f"{root}/projects/{proj}", exist_ok=True)
    with open(f"{root}/projects/{proj}/{sid}.jsonl", "w") as f:
        f.write(json.dumps({"type": "user", "parentUuid": None,
                            "message": {"role": "user", "content": title},
                            "cwd": cwd, "sessionId": sid, "gitBranch": branch,
                            "version": "2.1.263"}) + "\n")
PY

pkill -f "MacOS/heron" 2>/dev/null || true; sleep 2
PATH="$DEMO/bin:$PATH" CLAUDE_CONFIG_DIR="$DEMO" "$APP" >/tmp/heron-demo.log 2>&1 &
sleep 6

# The tray icon's position differs per machine; pass TRAY_X / TRAY_Y to override.
TRAY_X=${TRAY_X:-1050}; TRAY_Y=${TRAY_Y:-12}
screencapture -V 12 /tmp/heron-demo.mov >/dev/null 2>&1 &
REC=$!
sleep 2; cliclick c:$TRAY_X,$TRAY_Y; sleep 3
cliclick m:$((TRAY_X - 120)),200; sleep 1.5
cliclick m:$((TRAY_X - 120)),260; sleep 1.5
cliclick kp:esc; sleep 2
wait $REC

# Crop to the app only: the recording is full-screen, the GIF must not be.
read -r X Y W H <<<"$(osascript -e 'tell application "System Events" to tell process "Heron" to get {position, size} of window 1' | tr ',' ' ')"
ffmpeg -y -i /tmp/heron-demo.mov -vf \
  "crop=$((W*2)):$((H*2)):$((X*2)):$((Y*2)),fps=12,scale=680:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse" \
  -loop 0 docs/media/demo.gif
pkill -f "MacOS/heron" 2>/dev/null || true
pkill -f "exec -a claude sleep" 2>/dev/null || true
rm -rf "$DEMO" /tmp/heron-demo.mov
echo "wrote docs/media/demo.gif"
