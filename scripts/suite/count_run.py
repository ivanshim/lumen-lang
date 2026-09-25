import sys, subprocess, pathlib, time
ROOT = pathlib.Path(__import__("os").environ.get("LUMEN_ROOT", ".")).resolve(); BIN = pathlib.Path(sys.argv[3]) if len(sys.argv) > 3 else ROOT/"target"/"debug"/"lumen-lang"
RAW = pathlib.Path(sys.argv[1]); RAW.mkdir(parents=True, exist_ok=True)
CAP = int(sys.argv[2]) if len(sys.argv) > 2 else 900
for p in sorted((ROOT/"tests"/"python").glob("*.py")):
    for k in ("stack8", "microcode7"):
        dest = RAW/f"{p.stem}.{k}.txt"
        if dest.exists(): continue
        t0 = time.time()
        try:
            r = subprocess.run([str(BIN),"--kernel",k,str(p)], capture_output=True,
                               text=True, timeout=CAP, stdin=subprocess.DEVNULL)
            dest.write_text("###EXIT %d\n###SECONDS %.1f\n" % (r.returncode, time.time()-t0)
                            + r.stdout + "\n###STDERR\n" + r.stderr)
        except subprocess.TimeoutExpired:
            dest.write_text("###EXIT -1\n###SECONDS %.1f\n###TIMEOUT after %d\n" % (time.time()-t0, CAP))
        print("done %s %s %.1fs" % (p.stem, k, time.time()-t0), flush=True)
