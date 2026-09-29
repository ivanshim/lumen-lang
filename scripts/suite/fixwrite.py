import sys, subprocess, pathlib
from stderr_record import measured_line
root = pathlib.Path(sys.argv[1]); binary = root/"target/debug/lumen-lang"
for p in sys.argv[2:]:
    py = root/"scratch"/(p + ".py"); err = py.with_suffix(".err")
    r = subprocess.run([str(binary), "--kernel", "stack8", str(py)], capture_output=True, text=True, timeout=900, stdin=subprocess.DEVNULL)
    out = py.with_suffix(".out")
    if out.exists():
        assert r.returncode == 0, p
        old = out.read_text()
        if r.stdout != old:
            out.write_text(r.stdout)
            print(p, "written", len(r.stdout), "was", len(old), flush=True)
        continue
    assert r.returncode != 0, p
    first = measured_line(r.stderr)
    assert first, p
    old = err.read_text() if err.exists() else None
    new = first + "\n"
    if new != old:
        err.write_text(new)
        print(p, "written", len(first), "was", None if old is None else len(old.rstrip("\n")), flush=True)
