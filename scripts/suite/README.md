# scripts/suite — checking the Python work

Small tools the coordinator and workers use to verify a change before it
is pushed (HANDOVER.md §3). Run them from a tree's root with its debug
build (`RUSTFLAGS="-D warnings" cargo build`). None of them edits a test.

| Script | What it does |
|---|---|
| `scratchcheck.py <root> <program.py>...` | Applies CI's `scratch` rule to each program on stack8 and microcode7: the exact stdout against `.out` (exit 0), or the measured stderr line against `.err` (non-zero exit). Prints only mismatches (and any file a program left behind), then `checked N programs, M mismatches`. One process per core (`SUITE_JOBS` to change), each program in its own directory of links to the tree, so `@test` files cannot meet; `SUITE_BIN` names a copied binary. About 10 minutes for every program on 16 cores. |
| `fixcheck.py <root> <piece/n>...` | Runs the named `scratch/` programs on both kernels and logs each measured stderr line, for `progcmp.py`. |
| `progcmp.py <fixcheck.log> <root>` | Compares each logged line with its record and with the other kernel: `agree=`, `stack8=OK/..`, `micro=OK/..`, and how many `.` became `E` or `F`. The dot count covers one kernel only; compare the other by hand. |
| `fixwrite.py <root> <piece/n>...` | Writes a moved `.out` record from a successful run's stdout, or an `.err` record from the current binary's measured stderr line. It leaves an identical record untouched. Use only once both kernels print the same new line and no `.` was lost on either. |
| `count_run.py <rawdir> <cap> <binary>` | Runs every `tests/python/*.py` once per kernel into `<rawdir>/<test>.<kernel>.txt` (exit, seconds, stdout, stderr), skipping files already written, so it can be restarted. Reads the tests from `$LUMEN_ROOT` (default: the current directory). One process per core (`SUITE_JOBS`), each file in its own working directory, largest first; about 5 minutes on 16 cores. |
| `rerun_one.py <rawdir> <test> <kernel> <cap> <binary>` | Reruns one reference file on one kernel with a longer cap (for a file load pushed past the cap). |
| `count.py <rawdir> <previous rawdir>` | Per kernel: pass and ran totals, and each file whose pass count changed. |
| `test-debug.sh --lang python\|php\|lumen` | `test.sh` on the debug binary with a 30 s cut-off per program: Python examples on stack8 and microcode7 only (reference kernels ignore ext.* arithmetic), requiring exit 0 and identical output; other languages on all six kernels, each compared with stream35. Expected: python 128, php 318, lumen 522. |

The shared `stderr_record.measured_line` rule uses the first stderr line.
When that line is exactly `Traceback (most recent call last):`, it uses
the last non-empty stderr line instead: the exception type and message.
The `.err` record contains that one line, while exit status must still be non-zero.

Copy the binary aside (`cp target/debug/lumen-lang /tmp/bin-<sha>`) before
a long run, and never rebuild a tree while a check runs on its binary.
`scratch/reader-tail/4.py` (`test_set`) takes 10–40 minutes per kernel;
run it alone with `SUITE_JOBS=1`. The checkers give this fixture a 2,400 s cap;
other programs keep the 900 s cap.

`worker-brief.md` is the template for a worker's brief.

`fixwrite.py --from-logs <json> <root> <piece/n>...` writes records from
reviewed measurements made elsewhere, including Lambda. The JSON maps each
piece to `stack8` and `microcode7` objects with `returncode`, `stdout`, and
`stderr`. Both kernels must agree under stderr_record.py's rule. This mode
also handles a justified change between successful and failing status; review
the source change and test identities before supplying such measurements.
