#!/usr/bin/env python3
# lsplit.py <package-dir-or-tests-dir> <test-name> <rawdir>: run one long reference file on Lambda ($FUNCTION) as one
# invocation per test class (LUMEN_UNITTEST_ONLY, which needs a binary with fix/unittest-only), both kernels, and
# write <rawdir>/<test>.<kernel>.txt with the classes' progress lines joined in the order a whole run takes them.
import json, os, pathlib, re, sys, time
from concurrent.futures import ThreadPoolExecutor, as_completed
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from python_versions import select
import boto3
from botocore.config import Config
src = pathlib.Path(sys.argv[1]); name = sys.argv[2]; raw = pathlib.Path(sys.argv[3]); raw.mkdir(parents=True, exist_ok=True)
classes = sorted(set(re.findall(r"^class (\w+)", (src / f"{name}.py").read_text(), re.M)))
version = select()
prog = f"{version['tests']}/{name}.py"
runs = [(prog, k, c) for k in ("stack8", "microcode7") for c in classes]
client = boto3.Session(profile_name=os.environ.get("LUMEN_AWS_PROFILE", "lumen-lambda"), region_name=os.environ.get("AWS_REGION", "ap-southeast-1")).client(
    "lambda", config=Config(read_timeout=910, connect_timeout=10, max_pool_connections=len(runs),
                            retries={"max_attempts": 6, "mode": "adaptive"}))
def go(r):
    x = client.invoke(FunctionName=os.environ["FUNCTION"], Payload=json.dumps({"count": [list(r)], "cap": 870}).encode())
    body = json.loads(x["Payload"].read())
    if "FunctionError" in x: raise RuntimeError(f"{r}: {body}")
    return body[0]
t0 = time.time(); res = {}
with ThreadPoolExecutor(len(runs)) as pool:
    for f in as_completed([pool.submit(go, r) for r in runs]):
        r = f.result(); res[(r["k"], r["only"])] = r
for k in ("stack8", "microcode7"):
    line = ""; ran = 0; secs = 0.0; bad = []; body = []
    for c in classes:
        r = res[(k, c)]; secs = max(secs, r["s"])
        if r["timeout"]: bad.append(f"{c}: TIMEOUT"); continue
        prog_lines = [l for l in r["err"].splitlines() if l and set(l) <= set(".FEsx")]
        m = re.search(r"^Ran (\d+) test", r["err"], re.M)
        if m and int(m.group(1)) > 0:
            line += prog_lines[0] if prog_lines else "?" * int(m.group(1)); ran += int(m.group(1))
            body.append(re.sub(r"^Ran \d+ tests? in .*$", "", r["err"], flags=re.M))   # one Ran line for the whole file
        elif r["rc"] != 0 and not m:
            bad.append(f"{c}: exit {r['rc']}: {r['err'].strip().splitlines()[-1:]}")
    (raw / f"{name}.{k}.txt").write_text("###EXIT 0\n###SECONDS %.1f\n\n###STDERR\n%s\nRan %d tests\n%s\n" % (secs, line, ran, "\n".join(body)))
    print(f"{name} {k} · CPython {version['release']} suite: {line.count('.')} pass of {ran}, {len(classes)} classes, slowest part {secs:.0f}s" + ("; PROBLEMS " + "; ".join(bad) if bad else ""))
print("wall %.0fs" % (time.time() - t0))
