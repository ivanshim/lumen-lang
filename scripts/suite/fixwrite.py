import sys, subprocess, pathlib
root = pathlib.Path(sys.argv[1]); binary = root/"target/debug/lumen-lang"
for p in sys.argv[2:]:
    py = root/"scratch"/(p + ".py"); err = py.with_suffix(".err")
    r = subprocess.run([str(binary), "--kernel", "stack8", str(py)], capture_output=True, text=True, timeout=900, stdin=subprocess.DEVNULL)
    assert r.returncode != 0, p
    first = r.stderr.splitlines()[0]
    old = err.read_text().splitlines()[0] if err.exists() else None
    err.write_text(first + "\n")
    print(p, "written", len(first), "was", None if old is None else len(old), flush=True)
