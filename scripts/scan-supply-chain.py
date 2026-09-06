#!/usr/bin/env python3
"""Scan this repository for injected dropper payloads and for network code.

Two jobs, because Heron makes two promises.

1. Nothing malicious has been injected into the tree or into any branch on the
   remote. Uses Python rather than grep: these payloads are a single line tens
   of thousands of characters long, which grep silently fails to match. Font
   files are checked against their magic bytes too, since a payload disguised
   with a .woff2 extension is otherwise skipped as a binary asset.

2. Heron still has no network code. That is the app's central claim, so a
   dependency or a patch that quietly adds an HTTP client should fail the build
   exactly like malware does.

Exit codes: 0 clean, 1 findings. Writes a GitHub step summary when run in CI.

    scripts/scan-supply-chain.py              # working tree + remote branches
    scripts/scan-supply-chain.py --local      # working tree only (fast, pre-commit)
"""

import os
import re
import subprocess
import sys

# --- 1. injected dropper payloads -------------------------------------------

MALWARE = [
    (r"global\.i\s*=\s*['\"]A[\d\-*#]", "injected global.i marker"),
    (r"wuqktamceigynzbosdctpusocrjhrflovnxrt", "known IOC string"),
    (r"Tgw\(2509\)", "known IOC call"),
    (r"_0x[0-9a-f]{4,6}", "obfuscated _0x identifiers"),
    (r"eth_getBlockByNumber|eth_getTransactionCount", "blockchain-resolved C2"),
    (r"spawn\([^)]{0,40}'-e'", "spawn('-e') dropper"),
    (r"'detached':\s*!!\[\]", "detached process"),
    # Generic droppers, whatever the language.
    (r"curl\s+-[a-zA-Z]*s[a-zA-Z]*\s+https?://[^\s|]+\s*\|\s*(ba)?sh", "curl piped into a shell"),
    (r"(?:eval|new\s+Function)\s*\(\s*(?:atob|Buffer\.from)\s*\(", "eval of decoded text"),
    (r"IEX\s*\(\s*New-Object\s+Net\.WebClient", "PowerShell download cradle"),
    (r"child_process[\s\S]{0,80}(?:exec|spawn)[\s\S]{0,80}(?:atob|base64)", "base64 into a process spawn"),
]

# --- 2. the no-network promise ----------------------------------------------
#
# Heron reads local files and talks to nothing. Anything here in shipped code is
# a regression, not a style question. Tests and this scanner are exempt.

NETWORK_RUST = [
    (r"\breqwest\b", "reqwest HTTP client"),
    (r"\bhyper\b", "hyper HTTP"),
    (r"\bstd::net\b|\bTcpStream\b|\bTcpListener\b|\bUdpSocket\b", "raw sockets"),
    (r"\btokio::net\b", "tokio networking"),
    (r"tauri_plugin_(?:http|updater)", "networked Tauri plugin"),
]
NETWORK_WEB = [
    (r"\bfetch\s*\(", "fetch()"),
    (r"\bXMLHttpRequest\b", "XMLHttpRequest"),
    (r"\bnew\s+WebSocket\b", "WebSocket"),
    (r"\bnavigator\.sendBeacon\b", "sendBeacon"),
    (r"\bEventSource\b", "EventSource"),
]

SCAN_EXT = (
    ".js", ".cjs", ".mjs", ".ts", ".tsx", ".jsx", ".svelte",
    ".rs", ".toml", ".py", ".sh", ".ps1", ".yml", ".yaml",
    ".woff", ".woff2", ".ttf", ".otf", ".eot", ".svg",
    ".json", ".html", ".css", ".map",
)

SKIP = (
    "node_modules/", "/dist/", "/build/", ".min.", "vendor/", "/.git/",
    "coverage/", "/target/", "/.svelte-kit/", "package-lock.json",
    "Cargo.lock", "/docs/media/", "/src-tauri/icons/", "/src-tauri/dmg/",
)

# Files allowed to mention network APIs: the ones that only talk *about* them.
NETWORK_EXEMPT = (
    "scripts/scan-supply-chain.py", "README.md", "docs/", ".claude/",
    "CLAUDE.md", "CONTRIBUTING.md",
)

# These two files quote the signatures above verbatim, so they match
# themselves. Their contents are reviewable in the repository like any other
# source file; skipping them here is what keeps the signal honest.
SELF = ("scripts/scan-supply-chain.py", "scripts/scan-machine.py")

FONT_MAGIC = {
    ".woff": (b"wOFF",),
    ".woff2": (b"wOF2",),
    ".ttf": (b"\x00\x01\x00\x00", b"true", b"ttcf"),
    ".otf": (b"OTTO", b"\x00\x01\x00\x00"),
}


def font_mismatch(path, data):
    """A font whose bytes are not a font is the disguise trick."""
    expected = FONT_MAGIC.get(os.path.splitext(path)[1].lower())
    if expected and data and not any(data.startswith(m) for m in expected):
        return "font extension but not font data"
    return None


def is_test(path, text):
    return "/tests/" in path or path.endswith("_test.rs") or "#[cfg(test)]" in text


def scan_path(path, data, check_network=True):
    hits = []
    mismatch = font_mismatch(path, data)
    if mismatch:
        hits.append(mismatch)
    if b"\0" in data[:4096]:
        return hits
    text = data.decode("utf-8", errors="ignore")
    if path not in SELF:
        hits += [name for pat, name in MALWARE if re.search(pat, text)]
    if not check_network or any(path.startswith(e) or e in path for e in NETWORK_EXEMPT):
        return hits
    if is_test(path, text):
        return hits
    rules = NETWORK_RUST if path.endswith(".rs") else NETWORK_WEB if path.endswith(
        (".ts", ".js", ".svelte", ".mjs", ".cjs")) else []
    hits += [f"network code: {name}" for pat, name in rules if re.search(pat, text)]
    return hits


def tracked_files(ref="HEAD"):
    out = subprocess.run(["git", "ls-tree", "-r", "--name-only", ref],
                         capture_output=True, text=True).stdout
    return [f for f in out.splitlines()
            if f.endswith(SCAN_EXT) and not any(s in "/" + f for s in SKIP)]


def blob(ref, path):
    r = subprocess.run(["git", "show", f"{ref}:{path}"], capture_output=True)
    return r.stdout if r.returncode == 0 else b""


def main():
    local_only = "--local" in sys.argv
    findings = []

    for path in tracked_files():
        if os.path.exists(path):
            with open(path, "rb") as fh:
                hits = scan_path(path, fh.read())
            if hits:
                findings.append(("working tree", path, hits))

    if not local_only:
        refs = subprocess.run(
            ["git", "for-each-ref", "--format=%(refname:short)", "refs/remotes/origin"],
            capture_output=True, text=True).stdout.split()
        for ref in refs:
            if ref.endswith("/HEAD"):
                continue
            for path in tracked_files(ref):
                hits = scan_path(path, blob(ref, path))
                if hits:
                    findings.append((ref, path, hits))

    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if findings:
        print("FINDINGS\n")
        for where, path, hits in findings:
            print(f"  {where}: {path}\n      -> {', '.join(hits)}")
            print(f"::error file={path}::{', '.join(hits)}")
        if summary:
            with open(summary, "a") as fh:
                fh.write("## :rotating_light: Scan found something\n\n")
                for where, path, hits in findings:
                    fh.write(f"- `{path}` ({where}) — {', '.join(hits)}\n")
        sys.exit(1)

    scope = "working tree" if local_only else "working tree and every origin branch"
    print(f"RESULT: CLEAN — no injected payloads and no network code in the {scope}.")
    if summary:
        with open(summary, "a") as fh:
            fh.write("## :lock: Scan clean\n\nNo injected payloads, and no network "
                     "code outside tests and documentation.\n")


if __name__ == "__main__":
    main()
