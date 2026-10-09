import os, sys, shutil, subprocess, pathlib, tempfile, time
from concurrent.futures import ThreadPoolExecutor, as_completed
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from python_versions import select, option
from python_tests import python_tests, python_test_name, python_test_args, python_test_module
from results_metadata import bind_for_run
REQUESTED = option(sys.argv)
# usage: count_run.py <rawdir> [cap] [binary]   one process per core (SUITE_JOBS to change), each
# file in its own working directory so two tests' @test files cannot meet; largest files first.
ROOT = pathlib.Path(os.environ.get("LUMEN_ROOT", ".")).resolve(); BIN = pathlib.Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else ROOT/"target"/"debug"/"lumen-lang"
RAW = pathlib.Path(sys.argv[1])
VERSION = select(REQUESTED, ROOT)
try:
    bind_for_run(RAW, VERSION)
except (ValueError, KeyError, OSError) as error:
    sys.exit(f"Error: {error}")
CAP = int(sys.argv[2]) if len(sys.argv) > 2 else 900
JOBS = int(os.environ.get("SUITE_JOBS", os.cpu_count()))

def run(p, k):
    dest = RAW/f"{python_test_name(p)}.{k}.txt"
    here = pathlib.Path(tempfile.mkdtemp(prefix="count-run-"))
    t0 = time.time()
    def go(argv):
        return subprocess.run([str(BIN),"--kernel",k,"--python",VERSION["release"]]+argv, capture_output=True,
                              text=True, timeout=CAP, stdin=subprocess.DEVNULL, cwd=here)
    try:
        r = go(python_test_args(p))
        module = python_test_module(p)
        if module and r.returncode and "relative imports require a package context" in r.stderr:
            r = go(["-m", module])
        dest.write_text("###EXIT %d\n###SECONDS %.1f\n" % (r.returncode, time.time()-t0)
                        + r.stdout + "\n###STDERR\n" + r.stderr)
    except subprocess.TimeoutExpired:
        dest.write_text("###EXIT -1\n###SECONDS %.1f\n###TIMEOUT after %d\n" % (time.time()-t0, CAP))
    shutil.rmtree(here)
    return time.time()-t0

tests = sorted(python_tests(ROOT, VERSION["tests"]), key=lambda p: -p.stat().st_size)
todo = [(p, k) for p in tests for k in ("stack8", "microcode7") if not (RAW/f"{python_test_name(p)}.{k}.txt").exists()]
with ThreadPoolExecutor(JOBS) as pool:
    runs = {pool.submit(run, p, k): (p, k) for p, k in todo}
    for done in as_completed(runs):
        (p, k) = runs[done]
        print("done %s %s %.1fs · CPython %s suite" % (python_test_name(p), k, done.result(), VERSION["release"]), flush=True)
