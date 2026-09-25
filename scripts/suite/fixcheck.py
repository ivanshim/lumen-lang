import sys, subprocess, pathlib, time
root = pathlib.Path(sys.argv[1]); binary = root/"target/debug/lumen-lang"
progs = sys.argv[2:]
for p in progs:
    py = root/"scratch"/(p + ".py")
    out, err = py.with_suffix(".out"), py.with_suffix(".err")
    for k in ("stack8", "microcode7"):
        t0 = time.time()
        try:
            r = subprocess.run([str(binary), "--kernel", k, str(py)], capture_output=True, text=True, timeout=900, stdin=subprocess.DEVNULL)
            status, so, se = r.returncode, r.stdout, r.stderr
        except subprocess.TimeoutExpired:
            status, so, se = 124, "", "TIMEOUT"
        first = se.splitlines()[0] if se.strip() else ""
        if out.exists():
            ok = status == 0 and so == out.read_text()
            want = "out"
        elif err.exists():
            ok = status != 0 and first == err.read_text().splitlines()[0]
            want = "err:" + err.read_text().splitlines()[0][:100]
        else:
            ok = False; want = "none"
        prog = [l for l in se.splitlines() if l and set(l) <= set(".FEsx")]
        line = prog[0] if prog else ""
        print("%-22s %-10s exit=%-3d %s %5.0fs want=%s\n    first=%s\n    prog=%s" % (p, k, status, "OK  " if ok else "DIFF", time.time()-t0, want, first[:140], line), flush=True)
