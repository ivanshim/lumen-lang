#!/usr/bin/env python3
# sweep.py <programs-list-file> [runs-per-batch] [max-concurrency]: the scratch rule on both kernels
# via the Lambda named in $FUNCTION, up to max-concurrency batches in flight (boto3 threads, one process).
import json, os, sys, time
from concurrent.futures import ThreadPoolExecutor, as_completed
import boto3
from botocore.config import Config

progs = [l.strip() for l in open(sys.argv[1]) if l.strip()]
per = int(sys.argv[2]) if len(sys.argv) > 2 else 3
width = int(sys.argv[3]) if len(sys.argv) > 3 else 900
runs = [(p, k) for p in progs for k in ("stack8", "microcode7")]
slow = [r for r in runs if r[0].endswith("reader-tail/4.py")]            # test_set, minutes long: alone
rest = [r for r in runs if r not in slow]
batches = [[r] for r in slow] + [rest[i:i + per] for i in range(0, len(rest), per)]

client = boto3.Session(profile_name=os.environ.get("LUMEN_AWS_PROFILE", "lumen-lambda"), region_name=os.environ.get("AWS_REGION", "ap-southeast-1")).client(
    "lambda", config=Config(read_timeout=910, connect_timeout=10, max_pool_connections=width,
                            retries={"max_attempts": 6, "mode": "adaptive"}))

def invoke(batch):
    r = client.invoke(FunctionName=os.environ.get("FUNCTION", "lumen-verify"), Payload=json.dumps({"runs": batch}).encode())
    body = json.loads(r["Payload"].read())
    if "FunctionError" in r:
        raise RuntimeError(f"{batch[0]}: {body}")
    return body

t0 = time.time(); bad = done = 0; secs = 0.0; slowest = (0, "")
with ThreadPoolExecutor(min(width, len(batches))) as pool:
    for fut in as_completed([pool.submit(invoke, b) for b in batches]):
        for r in fut.result():
            done += 1; secs += r["s"]; slowest = max(slowest, (r["s"], r["p"] + " " + r["k"]))
            if not r["ok"]:
                bad += 1
                print("MISMATCH %s %s exit=%d first=%s" % (r["p"], r["k"], r["rc"], r["first"][:120]), flush=True)
print("checked %d programs, %d mismatches" % (len(progs), bad))
print("runs %d in %d batches (up to %d at once); wall %.0fs; summed run time %.0fs; slowest %.0fs %s"
      % (done, len(batches), width, time.time() - t0, secs, slowest[0], slowest[1]))
