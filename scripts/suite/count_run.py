import os, sys, shutil, subprocess, pathlib, tempfile, time
from concurrent.futures import ThreadPoolExecutor, as_completed
# usage: count_run.py <rawdir> [cap] [binary]   one process per core (SUITE_JOBS to change), each
# file in its own working directory so two tests' @test files cannot meet; largest files first.
ROOT = pathlib.Path(os.environ.get("LUMEN_ROOT", ".")).resolve(); BIN = pathlib.Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else ROOT/"target"/"debug"/"lumen-lang"
RAW = pathlib.Path(sys.argv[1]); RAW.mkdir(parents=True, exist_ok=True)
CAP = int(sys.argv[2]) if len(sys.argv) > 2 else 900
JOBS = int(os.environ.get("SUITE_JOBS", os.cpu_count()))

def run(p, k):
    dest = RAW/f"{p.stem}.{k}.txt"
    here = pathlib.Path(tempfile.mkdtemp(prefix="count-run-"))
    t0 = time.time()
    try:
        r = subprocess.run([str(BIN),"--kernel",k,str(p)], capture_output=True,
                           text=True, timeout=CAP, stdin=subprocess.DEVNULL, cwd=here)
        dest.write_text("###EXIT %d\n###SECONDS %.1f\n" % (r.returncode, time.time()-t0)
                        + r.stdout + "\n###STDERR\n" + r.stderr)
    except subprocess.TimeoutExpired:
        dest.write_text("###EXIT -1\n###SECONDS %.1f\n###TIMEOUT after %d\n" % (time.time()-t0, CAP))
    shutil.rmtree(here)
    return time.time()-t0

tests = sorted((ROOT/"tests"/"python").glob("*.py"), key=lambda p: -p.stat().st_size)
todo = [(p, k) for p in tests for k in ("stack8", "microcode7") if not (RAW/f"{p.stem}.{k}.txt").exists()]
with ThreadPoolExecutor(JOBS) as pool:
    runs = {pool.submit(run, p, k): (p, k) for p, k in todo}
    for done in as_completed(runs):
        (p, k) = runs[done]
        print("done %s %s %.1fs" % (p.stem, k, done.result()), flush=True)
