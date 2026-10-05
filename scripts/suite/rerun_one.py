import sys, subprocess, time, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from python_versions import select, option
from python_tests import python_test_path
from results_metadata import bind_for_run
VERSION = select(option(sys.argv), pathlib.Path(__import__('os').environ.get('LUMEN_ROOT', '.')).resolve())
raw, stem, k, cap, binary = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5]
try:
    bind_for_run(raw, VERSION)
except (ValueError, KeyError, OSError) as error:
    sys.exit(f"Error: {error}")
p = python_test_path(pathlib.Path(__import__("os").environ.get("LUMEN_ROOT", ".")).resolve(), stem, VERSION["tests"]); dest = raw/f"{stem}.{k}.txt"; t0 = time.time()
try:
    r = subprocess.run([binary,"--kernel",k,"--python",VERSION["release"],str(p)], capture_output=True, text=True, timeout=cap, stdin=subprocess.DEVNULL)
    dest.write_text("###EXIT %d\n###SECONDS %.1f\n" % (r.returncode, time.time()-t0) + r.stdout + "\n###STDERR\n" + r.stderr)
except subprocess.TimeoutExpired:
    dest.write_text("###EXIT -1\n###SECONDS %.1f\n###TIMEOUT after %d\n" % (time.time()-t0, cap))
print("CPython", VERSION["release"], "suite · done", stem, k, "%.1fs" % (time.time()-t0), flush=True)
