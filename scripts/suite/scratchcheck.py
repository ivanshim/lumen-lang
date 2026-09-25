import sys, subprocess, pathlib
# usage: scratchcheck.py <repo> <program.py>...   applies ci.yml's rule to each program on both kernels
root = pathlib.Path(sys.argv[1]); binary = root/"target/debug/lumen-lang"
bad = 0
for p in sys.argv[2:]:
    py = root/p; out, err = py.with_suffix(".out"), py.with_suffix(".err")
    for k in ("stack8", "microcode7"):
        try:
            r = subprocess.run([str(binary), "--kernel", k, str(py)], capture_output=True, text=True, timeout=900, stdin=subprocess.DEVNULL)
            status, so, se = r.returncode, r.stdout, r.stderr
        except subprocess.TimeoutExpired:
            status, so, se = 124, "", "TIMEOUT"
        first = se.splitlines()[0] if se.strip() else ""
        if out.exists(): ok = status == 0 and so == out.read_text()
        elif err.exists(): ok = status != 0 and first == (err.read_text().splitlines() or [""])[0]
        else: ok = False
        if not ok:
            bad += 1
            print("MISMATCH %s %s exit=%d first=%s" % (p, k, status, first[:120]), flush=True)
print("checked %d programs, %d mismatches" % (len(sys.argv) - 2, bad), flush=True)
