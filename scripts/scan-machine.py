#!/usr/bin/env python3
"""Walk a whole machine (or any folder) looking for injected dropper payloads.

The repository scanner (`scan-supply-chain.py`) only sees files git tracks in
this project. This one walks the filesystem instead, so it also covers other
checkouts, `node_modules`, build output and anything downloaded, which is
exactly where an injected payload hides.

    scripts/scan-machine.py                 # scan $HOME
    scripts/scan-machine.py /path /other    # scan specific roots
    scripts/scan-machine.py --quiet         # findings only

Prints a per-root summary and exits 1 if anything matched.
"""

import os
import re
import sys
import time

SIGNATURES = [
    (r"global\.i\s*=\s*['\"]A[\d\-*#]", "injected global.i marker"),
    (r"wuqktamceigynzbosdctpusocrjhrflovnxrt", "known IOC string"),
    (r"Tgw\(2509\)", "known IOC call"),
    (r"eth_getBlockByNumber|eth_getTransactionCount", "blockchain-resolved C2"),
    (r"spawn\([^)]{0,40}'-e'", "spawn('-e') dropper"),
    (r"'detached':\s*!!\[\]", "detached process"),
    (r"curl\s+-[a-zA-Z]*s[a-zA-Z]*\s+https?://[^\s|]+\s*\|\s*(ba)?sh", "curl piped into a shell"),
    (r"IEX\s*\(\s*New-Object\s+Net\.WebClient", "PowerShell download cradle"),
]

# `_0x1234` alone is what every minifier emits, so on a whole-machine sweep it
# is only interesting next to a second signal.
WEAK = [(r"_0x[0-9a-f]{4,6}", "obfuscated _0x identifiers")]

SCAN_EXT = (
    ".js", ".cjs", ".mjs", ".ts", ".tsx", ".jsx", ".svelte", ".vue", ".dart",
    ".woff", ".woff2", ".ttf", ".otf", ".eot", ".svg",
    ".json", ".html", ".css", ".map", ".sh", ".ps1", ".py", ".rb",
)

# Directories with nothing a payload could execute from, or that are too large
# to be worth the wall clock.
SKIP_DIRS = {
    ".git", ".svn", ".hg", "Library/Caches", "Library/Containers",
    "Library/Group Containers", "Library/Mobile Documents", ".Trash",
    "Pictures/Photos Library.photoslibrary", "Music", "Movies",
    ".cargo/registry", ".rustup", ".gradle", "DerivedData", ".venv", "venv",
    "__pycache__", ".mypy_cache", ".pytest_cache", "Applications",
}

FONT_MAGIC = {
    ".woff": (b"wOFF",),
    ".woff2": (b"wOF2",),
    ".ttf": (b"\x00\x01\x00\x00", b"true", b"ttcf"),
    ".otf": (b"OTTO", b"\x00\x01\x00\x00"),
}

MAX_BYTES = 8 * 1024 * 1024  # a dropper is one long line, not a huge asset


def font_mismatch(path, data):
    expected = FONT_MAGIC.get(os.path.splitext(path)[1].lower())
    if expected and data and not any(data.startswith(m) for m in expected):
        return "font extension but not font data"
    return None


def scan_file(path):
    try:
        size = os.path.getsize(path)
        if size == 0 or size > MAX_BYTES:
            return []
        with open(path, "rb") as fh:
            data = fh.read(MAX_BYTES)
    except OSError:
        return []
    hits = []
    mismatch = font_mismatch(path, data)
    if mismatch:
        hits.append(mismatch)
    if b"\0" in data[:4096]:
        return hits
    text = data.decode("utf-8", errors="ignore")
    strong = [name for pat, name in SIGNATURES if re.search(pat, text)]
    hits += strong
    if strong:
        hits += [name for pat, name in WEAK if re.search(pat, text)]
    return hits


def walk(root, quiet):
    findings, scanned, started = [], 0, time.time()
    root = os.path.abspath(os.path.expanduser(root))
    for dirpath, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        rel = os.path.relpath(dirpath, root)
        dirnames[:] = [
            d for d in dirnames
            if d not in SKIP_DIRS
            and os.path.join(rel, d).lstrip("./") not in SKIP_DIRS
            and not os.path.islink(os.path.join(dirpath, d))
        ]
        for name in filenames:
            if not name.endswith(SCAN_EXT):
                continue
            path = os.path.join(dirpath, name)
            scanned += 1
            if not quiet and scanned % 20000 == 0:
                print(f"  ... {scanned} files, {time.time() - started:.0f}s", flush=True)
            hits = scan_file(path)
            if hits:
                findings.append((path, hits))
                print(f"  HIT {path}\n      -> {', '.join(hits)}", flush=True)
    return findings, scanned, time.time() - started


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    quiet = "--quiet" in sys.argv
    roots = args or [os.path.expanduser("~")]
    total, scanned_all = [], 0
    for root in roots:
        print(f"scanning {root} ...", flush=True)
        findings, scanned, secs = walk(root, quiet)
        scanned_all += scanned
        total += findings
        print(f"  {scanned} files in {secs:.0f}s, {len(findings)} finding(s)\n", flush=True)
    if total:
        print(f"FINDINGS: {len(total)} file(s) matched across {scanned_all} scanned.")
        sys.exit(1)
    print(f"RESULT: CLEAN — {scanned_all} files scanned, nothing matched.")


if __name__ == "__main__":
    main()
