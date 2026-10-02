import sys, subprocess, pathlib, json
from types import SimpleNamespace
from stderr_record import measured_line
args = sys.argv[1:]
logs = None
if args[0] == '--from-logs':
    logs = json.loads(pathlib.Path(args[1]).read_text())
    args = args[2:]
root = pathlib.Path(args[0]); binary = root/"target/debug/lumen-lang"
for p in args[1:]:
    py = root/"scratch"/(p + ".py"); err = py.with_suffix(".err")
    if logs is None:
        r = subprocess.run([str(binary), "--kernel", "stack8", str(py)], capture_output=True, text=True, timeout=900, stdin=subprocess.DEVNULL)
    else:
        pair = logs[p]
        a, b = pair['stack8'], pair['microcode7']
        assert (a['returncode'] == 0) == (b['returncode'] == 0), p
        if a['returncode'] == 0:
            assert a['stdout'] == b['stdout'], p
        else:
            assert measured_line(a['stderr']) == measured_line(b['stderr']), p
        r = SimpleNamespace(**a)
        # Logs permit status changes only after the caller has reviewed their cause.
        obsolete = py.with_suffix('.err' if r.returncode == 0 else '.out')
        if obsolete.exists():
            obsolete.unlink()
        if r.returncode == 0 and not py.with_suffix('.out').exists():
            py.with_suffix('.out').write_text('')
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
