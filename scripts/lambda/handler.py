# lumen-verify: run a batch of scratch programs and apply CI's scratch rule to each.
# The interpreter is started through the bundled loader and C library (sys/), so it runs
# with the same glibc and libm as the build box.
import os, shutil, subprocess, tempfile, time

ROOT = os.environ.get("LAMBDA_TASK_ROOT", ".")
RUN = [f"{ROOT}/sys/ld-linux-aarch64.so.1", "--library-path", f"{ROOT}/sys", f"{ROOT}/lumen-lang"]
# The binary names its library files by the absolute path of the checkout it was built in
# (/tmp/lvsrc/lumen-lang); only /tmp is writable here, so that path is made to point at this package.
BUILT_AT = "/tmp/lvsrc/lumen-lang"
if not os.path.exists(BUILT_AT):
    os.makedirs(os.path.dirname(BUILT_AT), exist_ok=True)
    os.symlink(ROOT, BUILT_AT)
# The same basic environment a program sees on the build box (Lambda sets almost none of it).
ENV = {"HOME": "/tmp", "USER": "rocky", "LANG": "en_US.UTF-8", "PATH": "/usr/local/bin:/usr/bin:/bin", "SHELL": "/bin/bash"}
ENV["LUMEN_ROOT"] = ROOT          # binaries with fix/relocatable-library read the library from here


def check(prog, kernel, cap):
    here = tempfile.mkdtemp(dir="/tmp")
    for top in ("scratch", "langs", "tests"):          # as scratchcheck.py links the tree's top level
        os.symlink(f"{ROOT}/{top}", f"{here}/{top}")
    started = time.time()
    try:
        r = subprocess.run(RUN + ["--kernel", kernel, prog], cwd=here, capture_output=True,
                           timeout=cap, stdin=subprocess.DEVNULL, env=ENV)
        status, out, err = r.returncode, r.stdout.decode("utf-8", "replace"), r.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        status, out, err = 124, "", "TIMEOUT"
    shutil.rmtree(here, ignore_errors=True)
    first = err.splitlines()[0] if err.strip() else ""
    base = f"{ROOT}/{prog[:-3]}"
    if os.path.exists(base + ".out"):
        ok = status == 0 and out == open(base + ".out", encoding="utf-8").read()
    elif os.path.exists(base + ".err"):
        want = (open(base + ".err", encoding="utf-8").read().splitlines() or [""])[0]
        ok = status != 0 and first == want
    else:
        ok = False
    return {"p": prog, "k": kernel, "ok": ok, "rc": status, "first": first[:200], "s": round(time.time() - started, 2), "tail": "" if ok else "\n".join(err.splitlines()[-12:])[-1500:]}


def handler(event, context):
    cap = event.get("cap", 840)
    if "source" in event:                      # probe mode: run a small program given inline
        os.makedirs("/tmp/probe", exist_ok=True)
        open("/tmp/probe/p.py", "w").write(event["source"])
        r = subprocess.run(RUN + ["--kernel", event.get("kernel", "stack8"), "/tmp/probe/p.py"], cwd="/tmp/probe",
                           capture_output=True, timeout=cap, stdin=subprocess.DEVNULL, env=ENV)
        return {"rc": r.returncode, "out": r.stdout.decode("utf-8", "replace")[-3000:], "err": r.stderr.decode("utf-8", "replace")[-3000:]}
    if "gate" in event:                        # gate mode: one example on one kernel, as test-debug.sh runs it
        return [gate(f, k, flag, cap) for f, k, flag in event["gate"]]
    if "count" in event:                       # count mode: a reference file's whole output, as count_run.py keeps it
        return [count(r[0], r[1], cap, r[2] if len(r) > 2 else None) for r in event["count"]]
    return [check(p, k, cap) for p, k in event["runs"]]


def clip(text, head=1500000, tail=1000000):
    # keep the progress line at the start and the summary at the end; the response limit is 6 MB
    return text if len(text) <= head + tail else text[:head] + "\n...[clipped]...\n" + text[-tail:]


def count(prog, kernel, cap, only=None):
    here = tempfile.mkdtemp(dir="/tmp")         # an empty working directory, as count_run.py gives each file
    started = time.time()
    try:
        env = dict(ENV, LUMEN_UNITTEST_ONLY=only) if only else ENV   # one part of a long file
        r = subprocess.run(RUN + ["--kernel", kernel, f"{BUILT_AT}/{prog}"], cwd=here, capture_output=True,
                           timeout=cap, stdin=subprocess.DEVNULL, env=env)
        res = {"rc": r.returncode, "out": r.stdout.decode("utf-8", "replace")[-2000000:],
               "err": clip(r.stderr.decode("utf-8", "replace")), "timeout": False}
    except subprocess.TimeoutExpired:
        res = {"rc": -1, "out": "", "err": "", "timeout": True}
    shutil.rmtree(here, ignore_errors=True)
    res.update({"p": prog, "k": kernel, "only": only, "s": round(time.time() - started, 1)})
    return res


def gate(file, kernel, flag, cap):
    here = tempfile.mkdtemp(dir="/tmp")         # the tree's top level linked in, so relative paths work
    for top in os.listdir(ROOT):
        if top in ("scratch", "langs", "tests", "examples", "kernels"):
            os.symlink(f"{ROOT}/{top}", f"{here}/{top}")
    started = time.time()
    try:
        r = subprocess.run(RUN + ["--kernel", kernel] + flag + [file], cwd=here, stdout=subprocess.PIPE,
                           stderr=subprocess.STDOUT, timeout=cap, stdin=subprocess.DEVNULL, env=ENV)
        rc, out = r.returncode, r.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        rc, out = 124, ""
    shutil.rmtree(here, ignore_errors=True)
    return {"f": file, "k": kernel, "rc": rc, "out": out[-200000:], "s": round(time.time() - started, 2)}
