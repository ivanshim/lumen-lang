import os, sys, shutil, subprocess, pathlib, tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed
# usage: scratchcheck.py <repo> <program.py>...   applies ci.yml's rule to each program on both kernels
# One process per core (SUITE_JOBS to change), each in its own directory of links to the tree's
# top level, so relative paths read as they do from the root in CI and files a program writes
# (@test) cannot meet another program's. SUITE_BIN names a copied binary.
root = pathlib.Path(sys.argv[1]).resolve()
binary = pathlib.Path(os.environ.get("SUITE_BIN", root/"target/debug/lumen-lang")).resolve()
jobs = int(os.environ.get("SUITE_JOBS", os.cpu_count()))

def check(p, k):
    py = root/p; out, err = py.with_suffix(".out"), py.with_suffix(".err")
    here = pathlib.Path(tempfile.mkdtemp(prefix="scratchcheck-"))
    for entry in root.iterdir(): (here/entry.name).symlink_to(entry)
    try:
        r = subprocess.run([str(binary), "--kernel", k, p], capture_output=True, text=True, timeout=900, stdin=subprocess.DEVNULL, cwd=here)
        status, so, se = r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        status, so, se = 124, "", "TIMEOUT"
    left = sorted(e.name for e in here.iterdir() if not e.is_symlink())
    shutil.rmtree(here)
    first = se.splitlines()[0] if se.strip() else ""
    if out.exists(): ok = status == 0 and so == out.read_text()
    elif err.exists(): ok = status != 0 and first == (err.read_text().splitlines() or [""])[0]
    else: ok = False
    return ok, status, first, left

programs = [str(pathlib.Path(p).resolve().relative_to(root)) if pathlib.Path(p).is_absolute() else p for p in sys.argv[2:]]
bad = 0
with ThreadPoolExecutor(jobs) as pool:
    runs = {pool.submit(check, p, k): (p, k) for p in programs for k in ("stack8", "microcode7")}
    for done in as_completed(runs):
        (p, k), (ok, status, first, left) = runs[done], done.result()
        if not ok:
            bad += 1
            print("MISMATCH %s %s exit=%d first=%s" % (p, k, status, first[:120]), flush=True)
        if left: print("LEFT BEHIND %s %s %s" % (p, k, " ".join(left)), flush=True)
print("checked %d programs, %d mismatches" % (len(programs), bad), flush=True)
