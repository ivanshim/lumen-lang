# scripts/suite — checking the Python work

Small tools the coordinator and workers use to verify a change before it
is pushed (HANDOVER.md §3). Run them from a tree's root with its debug
build (`RUSTFLAGS="-D warnings" cargo build`). None of them edits a test.

| Script | What it does |
|---|---|
| `scratchcheck.py <root> <program.py>...` | Applies CI's `scratch` rule to each program on stack8 and microcode7: the exact stdout against `.out` (exit 0), or the exact first stderr line against `.err` (non-zero exit). Prints only mismatches (and any file a program left behind), then `checked N programs, M mismatches`. One process per core (`SUITE_JOBS` to change), each program in its own directory of links to the tree, so `@test` files cannot meet; `SUITE_BIN` names a copied binary. About 10 minutes for every program on 16 cores. |
| `fixcheck.py <root> <piece/n>...` | Runs the named `scratch/` programs on both kernels and logs each first stderr line, for `progcmp.py`. |
| `progcmp.py <fixcheck.log> <root>` | Compares each logged line with its record and with the other kernel: `agree=`, `stack8=OK/..`, `micro=OK/..`, and how many `.` became `E` or `F`. The dot count covers one kernel only; compare the other by hand. |
| `fixwrite.py <root> <piece/n>...` | Writes a moved `.err` record from the current binary's first stderr line. Use only once both kernels print the same new line and no `.` was lost on either. |
| `count_run.py <rawdir> <cap> <binary>` | Runs every `tests/python/*.py` once per kernel into `<rawdir>/<test>.<kernel>.txt` (exit, seconds, stdout, stderr), skipping files already written, so it can be restarted. Reads the tests from `$LUMEN_ROOT` (default: the current directory). One process per core (`SUITE_JOBS`), each file in its own working directory, largest first; about 5 minutes on 16 cores. |
| `rerun_one.py <rawdir> <test> <kernel> <cap> <binary>` | Reruns one reference file on one kernel with a longer cap (for a file load pushed past the cap). |
| `count.py <rawdir> <previous rawdir>` | Per kernel: pass and ran totals, and each file whose pass count changed. |
| `test-debug.sh --lang python\|php\|lumen` | `test.sh` on the debug binary with a 30 s cut-off per program: every example on all six kernels, each compared with stream35. Expected: python 384, php 318, lumen 522. |

Copy the binary aside (`cp target/debug/lumen-lang /tmp/bin-<sha>`) before
a long run, and never rebuild a tree while a check runs on its binary.
`scratch/reader-tail/4.py` (`test_set`) takes 10–40 minutes per kernel;
run it alone with a long cap rather than inside a sweep.

`worker-brief.md` is the template for a worker's brief.
