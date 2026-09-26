#!/usr/bin/env python3
# lcount.py <tests-list-file> <rawdir>: run reference files on both kernels through the Lambda in $FUNCTION,
# one file per invocation, all at once; write each result as count_run.py does (<rawdir>/<test>.<kernel>.txt).
import json, os, pathlib, sys, time
from concurrent.futures import ThreadPoolExecutor, as_completed
import boto3
from botocore.config import Config
tests = [l.strip() for l in open(sys.argv[1]) if l.strip()]
raw = pathlib.Path(sys.argv[2]); raw.mkdir(parents=True, exist_ok=True)
runs = [(t, k) for t in tests for k in ("stack8", "microcode7")]
client = boto3.Session(profile_name=os.environ.get("LUMEN_AWS_PROFILE", "lumen-lambda"), region_name=os.environ.get("AWS_REGION", "ap-southeast-1")).client(
    "lambda", config=Config(read_timeout=910, connect_timeout=10, max_pool_connections=len(runs) or 1,
                            retries={"max_attempts": 6, "mode": "adaptive"}))
def go(r):
    x = client.invoke(FunctionName=os.environ["FUNCTION"], Payload=json.dumps({"count": [r], "cap": 870}).encode())
    body = json.loads(x["Payload"].read())
    if "FunctionError" in x: raise RuntimeError(f"{r}: {body}")
    return body[0]
t0 = time.time(); slow = (0, "")
with ThreadPoolExecutor(len(runs) or 1) as pool:
    for f in as_completed([pool.submit(go, r) for r in runs]):
        r = f.result(); stem = pathlib.Path(r["p"]).stem; dest = raw / f"{stem}.{r['k']}.txt"
        if r["timeout"]:
            dest.write_text("###EXIT -1\n###SECONDS %.1f\n###TIMEOUT after 870\n" % r["s"])
        else:
            dest.write_text("###EXIT %d\n###SECONDS %.1f\n" % (r["rc"], r["s"]) + r["out"] + "\n###STDERR\n" + r["err"])
        slow = max(slow, (r["s"], f"{stem} {r['k']}"))
print("counted %d runs; wall %.0fs; slowest %.0fs %s" % (len(runs), time.time() - t0, *slow))
