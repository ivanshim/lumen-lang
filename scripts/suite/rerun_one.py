import sys, subprocess, time, pathlib
raw, stem, k, cap, binary = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5]
p = pathlib.Path(__import__("os").environ.get("LUMEN_ROOT", ".")).resolve()/"tests"/"python"/(stem+".py"); dest = raw/f"{stem}.{k}.txt"; t0 = time.time()
try:
    r = subprocess.run([binary,"--kernel",k,str(p)], capture_output=True, text=True, timeout=cap, stdin=subprocess.DEVNULL)
    dest.write_text("###EXIT %d\n###SECONDS %.1f\n" % (r.returncode, time.time()-t0) + r.stdout + "\n###STDERR\n" + r.stderr)
except subprocess.TimeoutExpired:
    dest.write_text("###EXIT -1\n###SECONDS %.1f\n###TIMEOUT after %d\n" % (time.time()-t0, cap))
print("done", stem, k, "%.1fs" % (time.time()-t0), flush=True)
