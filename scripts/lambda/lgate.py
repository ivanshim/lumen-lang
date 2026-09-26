#!/usr/bin/env python3
# lgate.py <examples-list-file> [cap=90]: test-debug.sh's rule on Lambda ($FUNCTION): every example on the six
# kernels, one invocation per run, all at once. A run passes when it exits 0 and prints exactly what stream35
# printed (trailing newlines ignored, as the shell's $(...) does). Runs over 30 s are listed as SLOW.
import json, os, sys, time, collections
from concurrent.futures import ThreadPoolExecutor, as_completed
import boto3
from botocore.config import Config
KERNELS = ["stream35", "microcode11", "microcode4", "microcode7", "stack5", "stack8"]
FLAG = {"php": ["--lang", "langs/php.json"]}
files = [l.strip() for l in open(sys.argv[1]) if l.strip()]
cap = int(sys.argv[2]) if len(sys.argv) > 2 else 90
lang = lambda f: f.split("/")[1]
runs = [(f, k, FLAG.get(lang(f), [])) for f in files for k in KERNELS]
client = boto3.Session(profile_name=os.environ.get("LUMEN_AWS_PROFILE", "lumen-lambda"), region_name=os.environ.get("AWS_REGION", "ap-southeast-1")).client(
    "lambda", config=Config(read_timeout=910, connect_timeout=10, max_pool_connections=800,
                            retries={"max_attempts": 6, "mode": "adaptive"}))
def go(r):
    x = client.invoke(FunctionName=os.environ["FUNCTION"], Payload=json.dumps({"gate": [r], "cap": cap}).encode())
    body = json.loads(x["Payload"].read())
    if "FunctionError" in x: raise RuntimeError(f"{r}: {body}")
    return body[0]
t0 = time.time(); res = {}
with ThreadPoolExecutor(800) as pool:
    for fut in as_completed([pool.submit(go, r) for r in runs]):
        r = fut.result(); res[(r["f"], r["k"])] = r
passed = collections.Counter(); total = collections.Counter(); bad = []; slow = []
for f in files:
    ref = res[(f, "stream35")]["out"].rstrip("\n")
    for k in KERNELS:
        r = res[(f, k)]; total[lang(f)] += 1
        if r["s"] > 30: slow.append(f"{f} {k} {r['s']}s")
        if k != "stream35" and r["out"].rstrip("\n") != ref: bad.append(f"DIFFERS {f} {k} (from stream35)")
        elif r["rc"] == 124: bad.append(f"TIMEOUT {f} {k} (> {cap}s)")
        elif r["rc"] != 0: bad.append(f"FAIL {f} {k} exit={r['rc']}: {r['out'].strip().splitlines()[-1:] }")
        else: passed[lang(f)] += 1
for b in bad: print(b)
for s in slow: print("SLOW", s)
print("gates:", ", ".join(f"{l} {passed[l]}/{total[l]}" for l in sorted(total)), "; wall %.0fs" % (time.time() - t0))
