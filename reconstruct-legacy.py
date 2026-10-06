#!/usr/bin/env python3
"""reconstruct-legacy.py [outdir]: rebuild the coordinator's retired text logs (astra-log.md, batches.txt, telemetry.tsv,
reviewed.tsv) byte for byte from the legacy-file / legacy-line events in events.jsonl, and check each against the
sha256 recorded when it was captured (2026-10-07)."""
import json, sys, hashlib, os, collections
out = sys.argv[1] if len(sys.argv) > 1 else "."
L = collections.defaultdict(dict); H = {}
for l in open(os.path.expanduser("~/coord/log-repo/events.jsonl"), encoding="utf-8", errors="surrogateescape"):
    e = json.loads(l)
    if e["event"] == "legacy-line": L[e["file"]][e["n"]] = e["text"]
    elif e["event"] == "legacy-file": H[e["file"]] = e
ok = True
for f, h in H.items():
    lines = [L[f][n] for n in range(1, h["lines"] + 1)]
    b = ("\n".join(lines) + ("\n" if h["final_newline"] else "")).encode("utf-8", "surrogateescape")
    open(os.path.join(out, f), "wb").write(b)
    match = hashlib.sha256(b).hexdigest() == h["sha256"]; ok &= match
    print(f"{f:14} {h['lines']:5} lines {len(b):7} bytes  sha256 {'MATCH' if match else 'DIFFERS'}")
sys.exit(0 if ok else 1)
