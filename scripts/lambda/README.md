# Checking a build on AWS Lambda

The checks that CI and the coordinator run on every branch — the scratch
records, the example gates and the count of the CPython reference files —
are many short, independent interpreter runs. On one machine they take hours;
on Lambda they run side by side, one run (or three) per invocation, several
hundred at once:

| Check | Runs | Wall time |
|---|---|---|
| scratch records, both full kernels | 2,328 | about 2 minutes |
| example gates (Python, PHP, Lumen), six kernels | 1,224 | about 1 minute |
| reference files, both full kernels (test_set by class) | 98 + 150 | about 12 minutes |

The results match the same checks run on the build machine exactly.

## How it works

`lverify.sh <worktree> <label>`, run where the AWS CLI and boto3 are set up:

1. On the build machine (reached by `ssh $LUMEN_BOX`), the worktree's own
   debug build is stripped and packed with the machine's dynamic loader and C
   library (Lambda's own C library is older), the handler, and the tree's
   `scratch/`, `langs/`, `examples/` and the suite directories from `langs/python/versions.json`.
2. A short-lived function `lumen-v-<label>` (Python 3.12 runtime, arm64,
   1,769 MB = one vCPU, 15-minute limit) is created from the package.
3. `sweep.py` applies CI's scratch rule (exact stdout for `.out` records, the
   first stderr line for `.err` records) to every program but
   `scratch/reader-tail/4.py` on stack8 and microcode7.
4. With `GATES=1`, `lgate.py` runs every example on the six kernels and
   applies `scripts/suite/test-debug.sh`'s rule: each kernel exits 0 and
   prints exactly what stream35 printed. The time limit there is 90 s (a
   Lambda vCPU is slower than the build machine's cores); runs over 30 s are
   listed as SLOW.
5. With `COUNT=1`, `lcount.py` runs every reference file but test_set on both
   kernels, one invocation each, and writes the outputs as
   `scripts/suite/count_run.py` does. test_set is too long for one invocation:
   `lsplit.py` runs it one test class per invocation
   (`LUMEN_UNITTEST_ONLY`, see `langs/lib_python/modules/unittest.py`) and
   joins the progress lines in the order a whole run takes them. With
   `BASE_COUNT` set, `scripts/suite/count.py` compares the result with an
   earlier count in `$WORK/base/`.
6. The function and its log group are deleted.

The build must include the `LUMEN_ROOT` override (the interpreter otherwise
looks for its library at the path it was built in).

## Setting up

- An IAM role for the functions with only the basic Lambda execution policy
  (`LUMEN_LAMBDA_ROLE`, default `lambda_basic_execution`).
- An AWS profile (`LUMEN_AWS_PROFILE`, default `lumen-lambda`) allowed only to
  create, invoke, read and delete functions named `lumen-v-*`, to pass that
  role, and to delete the `/aws/lambda/lumen-v-*` log groups.
- `LUMEN_BOX`: the ssh name of the arm64 build machine; `AWS_REGION`
  (default ap-southeast-1); `PYTHON`, a python3 with boto3; `WORK`, a
  scratch directory for packages and counts (default /tmp/lumen-lambda).

A sweep costs about US$0.27; gates and a count about US$0.50 more.

Reference counts use one output directory per full release and pass that
release via `--python`; count names include `CPython x.y.z suite`.
