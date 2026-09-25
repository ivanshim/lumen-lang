# Handover — read this and begin

This is the whole briefing for picking up `lumen-lang`'s Python work. It
says where things stand (§1), how the work is run (§2), how a change is
verified (§3), what went wrong before so it is not done twice (§4), and
how the machine the work now runs on is set up (§5). The batch-by-batch
diary that used to live here is in `HISTORY.md`; read it when a question
starts with "why is it like this".

**Branch for work:** `claude/codebase-familiarization-t6vjhi`, reset to
`main` after each batch merges. **Goal:** all 50 files in `tests/python/`
(CPython 3.14's own tests) pass on both full kernels, `stack8` and
`microcode7`, without weakening a test or a fixture.

---

## 1. Where things stand

**Main** holds everything through batch 18 (see the pull requests
#489 onward for each batch's contents; `HISTORY.md` §1k onward and the
PR bodies say what each one did). No branch holds unmerged work.

**PHP** is unchanged throughout: `pass 399, differs 0, error 0, skipped
23`. **The examples** agree on all six kernels. **No kernel
disagreements** in the reference run.

### The Python count

A file's count is the number of its tests that pass when the file is run
alone under the full `unittest` (`scripts/suite/count_run.py`, a 900 s
cap per file per kernel; `scripts/suite/count.py` compares two runs).

Measured at batch 18 (the build of a43713b), each file alone with a 900 s cap; pass/ran per file.

| File | stack8 | microcode7 |
|---|---|---|
| test_augassign | 5/7 | 5/7 |
| test_bigmem | runs nothing | runs nothing |
| test_binop | 3/12 | 3/12 |
| test_bool | 24/31 | 24/31 |
| test_builtin | 46/143 | 46/143 |
| test_class | 16/42 | 16/42 |
| test_cmath | 13/33 | 13/33 |
| test_compare | 14/16 | 14/16 |
| test_complex | 21/37 | 19/37 |
| test_contains | 4/4 | 4/4 |
| test_decorators | 12/16 | 12/16 |
| test_dict | 80/142 | 81/142 |
| test_dictcomps | 9/11 | 9/11 |
| test_enumerate | 77/105 | 77/105 |
| test_eof | 0/6 | 0/6 |
| test_exceptions | 32/115 | 32/115 |
| test_float | 21/54 | 21/54 |
| test_format | 6/18 | 6/18 |
| test_fractions | 19/50 | 19/50 |
| test_fstring | 45/92 | 45/92 |
| test_funcattrs | 13/39 | 13/39 |
| test_generators | 25/59 | 25/59 |
| test_genexps | 0/1 | 0/1 |
| test_global | 13/20 | 13/20 |
| test_grammar | 47/75 | 47/75 |
| test_index | 37/55 | 36/55 |
| test_int | 15/52 | 15/52 |
| test_int_literal | 6/6 | 6/6 |
| test_iter | 36/68 | 36/68 |
| test_keywordonlyarg | 7/11 | 7/11 |
| test_list | 45/71 | 44/71 |
| test_listcomps | 42/68 | 42/68 |
| test_long | 25/45 | 25/45 |
| test_math | 51/87 | 51/87 |
| test_opcodes | 5/8 | 5/8 |
| test_positional_only_arg | 5/28 | 5/28 |
| test_pow | 4/7 | 4/7 |
| test_print | 2/9 | 2/9 |
| test_range | 17/29 | 17/29 |
| test_scope | 32/41 | 32/41 |
| test_set | 490/644 | 490/644 |
| test_setcomps | 1/2 | 1/2 |
| test_slice | 8/11 | 8/11 |
| test_str | 81/138 | 90/138 |
| test_string_literals | 14/20 | 14/20 |
| test_syntax | 6/109 | 6/109 |
| test_tuple | 26/38 | 26/38 |
| test_unary | 6/6 | 6/6 |
| test_unpack | 1/2 | 1/2 |
| test_with | 38/55 | 40/55 |
| **All** | **1,545/2,738** | **1,553/2,738** |

Three files pass whole on both kernels: `test_contains` (4 of 4),
`test_int_literal` (6 of 6) and `test_unary` (6 of 6). The next closest
are `test_enumerate` (77 of 105, every other test skipped by CPython's own
conditions) and `test_with`.

### What is known to be missing (the next subjects)

Largest first, by how many tests each blocks. Each wants its own branch.

- **Run-time source lines.** Neither kernel tracks source lines while
  running, so there are no tracebacks (`__traceback__`, `tb_lineno`,
  `sys.excepthook` output) and `SyntaxError` has no `lineno`/`offset`/
  `text`. Most of `test_exceptions`' remaining failures, and part of
  `test_syntax`, wait on this. It is foundational work in both kernels'
  call machinery.
- **f-string expression fields.** `f-string: expecting '}'`, `valid
  expression required before '!'` and the like still read `invalid
  syntax`: the reader tokenizes the whole string before parsing, where
  CPython 3.12+ parses the fields while lexing. Prefix, unterminated and
  line-continuation wording is done.
- **`test_set`** (490 of 644 on both kernels) — not yet looked at as a subject.
- **Astral `\U` escapes.** The string-literal reader refuses every `\U`
  above the BMP, not only lone surrogates (`test_isprintable`,
  `test_surrogates`, `test_comparison`, `test_codecs_utf8` in test_str).
- **Codecs**: "this bytes operation is not supported" (test_str's
  `test_codecs*`, invalid-byte tests).
- **`compile()`/`eval()` "source operation cannot run yet"** (6 of
  test_builtin), `type('A', (int,), {...})` with a builtin base.
- **IntEnum/StrEnum** are stubs (`enum.py`), so `%` with an IntEnum and
  anything that subclasses both a builtin and Enum fails.
- **Pickling** of `zip`/`map`/`filter` and `str` iterators (enumerate,
  reversed and range iterators pickle now).
- **`collections.abc.Iterator`** does not recognise the kernels' native
  walks (several test_iter failures).
- **microcode7 only:** a bare `raise` from a resumed generator that must
  pass out through a plain function call ends `module did not finish`.
- Smaller, recorded where met: `re` error messages lack ` at position N`;
  `Exception.__setstate__`; `UnicodeEncodeError.__new__` without
  `__init__`; `signbit(-nan)` (NaN carries no sign); `isclose` bound as a
  class attribute becomes a method; CPython 3.14's newer math domain
  wording; `(detected at line N)` on unterminated strings; `exec()` with
  a custom `__builtins__`; `os.listdir` on a file should raise
  `NotADirectoryError`; `del f()[k]`; metaclass `__prepare__`;
  `type.__dict__` as a mappingproxy; generators are never finalised; lone
  surrogates; `reversed()` on a class with `__reversed__ = None` says
  "special method returned an invalid value"; `frozendict` (3.15).
- `test_bigmem` runs nothing (CPython's own decorator skips without a
  memory flag) and will count as 0 until that is decided.

### First hour on a new machine

1. Build (`RUSTFLAGS="-D warnings" cargo build`) and run `python3
   scripts/kernel_independence.py` (must end `0 problem(s)`).
2. Run the full scratch check and the count (§3) and compare with the
   last CI run and the table above. The work moved from x86 to ARM (§5):
   the system C library's math functions can differ in the last digit
   between the two, which would show as a scratch record or a test count
   that matches here but not in CI, or the other way round. Settle any
   such difference before trusting the machine.
3. Make `scripts/suite/scratchcheck.py` and `count_run.py` run their
   programs in parallel (one process per core, results written per
   program as they finish). They are sequential, which on the old 4-core
   container took 1.5–2 hours each; on 16 cores the whole verification
   should fit in well under half an hour. A release build is practical on
   this machine too, and checks on it run about ten times faster.

---

## 2. How the work is run

**One coordinator, a few workers, one pull request at a time.** The
coordinator (a Claude Code session running Opus) picks subjects from §1,
writes each worker a brief, verifies what each worker reports, merges the
verified branches into one integration branch per batch, runs the full
verification (§3), writes the batch's section into `HISTORY.md`, pushes
the batch to the working branch, opens one pull request, and merges it
when CI is green. Workers are Sonnet sessions spawned by the coordinator,
each in its own `git worktree` on its own `fix/<subject>` branch, **up
to four at a time** on the c8g.4xlarge. The owner raised this from two
(the limit in the old cloud session, which had 4 cores and a tight token
budget) when the work moved to the instance. Drop back to two if the
seven-day rate limit reaches `allowed_warning`, and say so in the
check-in summary. Each worker's build takes about 1.1 GB of disk and a
few GB of memory; four fit comfortably in 16 cores and 32 GB.

**A worker's brief** is `scripts/suite/worker-brief.md` with the subject
paragraph filled in: what fails, which reference tests show it, what
CPython does, what to leave alone. Every brief says: both kernels, each
in its own idiom; the five places a new Python-only label goes; the
gates; how records move; signed commits; never kill a process by
pattern; report only results from its own worktree; one final report.

**Verify every worker claim yourself.** Reports have been wrong in every
direction: a gate "passed" that was another run's log; a mismatch called
pre-existing that the branch's own base had introduced; a record moved
by hand from a partial run. The coordinator reruns the worker's probe on
both kernels against CPython's output, checks every moved record on both
kernels, reruns the reference tests the worker names, and runs the
independence check, before merging.

**The owner's standing rules.**
- Every commit is signed (`git cat-file -p <hash> | grep -c gpgsig` is 1).
  Coordinator commits end with the coordinator's `Co-Authored-By` and
  `Claude-Session` lines; worker commits keep their own. Pull-request
  bodies end with the Claude Code line and the session URL. No model or
  AI names anywhere else.
- Never weaken a test or a fixture; `tests/python/` is never edited.
- One open pull request at a time; batch verified fixes into it.
- Wait on long runs by running them in the background, not in polling
  loops; check in at most every 40 minutes.
- A summary at each check-in in Singapore time: what moved, what runs,
  the count, the pull request, the rate limit, and an instance-size
  recommendation (below).

**Instance size at each check-in.** End every summary with one line:
*Instance: keep c8g.4xlarge*, *Instance: suggest c8g.8xlarge*, or
*Instance: suggest c8g.2xlarge*, with the reason in a few words. Base it
on the last hour, from `sar -q` (load average) and `sar -r` (memory)
(sysstat, §5), plus what the work was waiting on:
- **Increase** (c8g.8xlarge, 32 cores, 64 GB, twice the price) when the
  machine is what holds work up: the 15-minute load average stayed
  above about 14 for most of the hour, memory available fell below
  about 4 GB or swap was used, or workers sat waiting on builds and
  checks rather than on CI or the model. Also when more than four
  workers are wanted.
- **Decrease** (c8g.2xlarge, 8 cores, 16 GB, half the price) when the
  machine mostly idles: the load average stayed below about 4 and memory
  used below about 12 GB for several check-ins in a row, with at most
  two workers running.
- **Keep** otherwise. Recommend a change only after it has held for at
  least two check-ins, so one busy or quiet hour does not flip it.

The coordinator only recommends; the owner resizes (stop the instance,
change its type, start it; the disk and all setup are kept). A resize
stops every process, Claude Code included, so first finish or stop the
running checks and workers and commit their work, and after the restart
start `claude remote-control` again (§5 step 6).

**How a scratch record moves.** Each `scratch/<piece>/<n>.py` has either
an `.out` (the exact stdout, exit 0) or an `.err` (the exact first line
of stderr, non-zero exit), and CI compares exactly that on both kernels.
A record moves only when both kernels print the same new line, CPython
agrees (or the suite's 3.14 wording does), and no `.` in a unittest
progress line became `E` or `F` on either kernel — compare position by
position. Write it with `scripts/suite/fixwrite.py`, never by hand.
Programs under `scratch/reader-tail/` run whole CPython test files;
`reader-tail/4` is `test_set` and takes 10–40 minutes per kernel, so it
is run alone with a long cap.

**How the code is shaped.**
- Two full kernels: `kernels/stack8/src/` (`compile.rs`, `engine.rs`,
  `classes.rs`, `value.rs`, `methods.rs`, `formatting.rs`, `lang.rs`, …)
  and `kernels/microcode7/src/` (`build.rs`, `exec.rs`, `classes.rs`,
  `data.rs`, `members.rs`, `table.rs`, `formatting.rs`, …). Four
  reference kernels read past every `ext.*` label. Every Python fix goes
  into both full kernels, each in its own words:
  `scripts/kernel_independence.py` refuses twelve identical significant
  lines between any two kernels.
- **A new Python-only label goes in five places:** `langs/python.json`;
  the `EXT_LABELS` roster, the `Lang` field and the reading line in
  `kernels/stack8/src/lang.rs`; the `EXT_TAGS` roster in
  `kernels/microcode7/src/table.rs` with its `:L`/`:B`/`:N` kind; prose
  in `langs/README.md`; then `python3 scripts/lang_table.py`. A new
  builtin in microcode7 also raises `BUILTIN_LABELS`'s declared length.
- **The exception roster is append-only.** `ext.builtin.exceptions` in
  `langs/python.json` is indexed by position in both kernels' parent
  maps (stack8 `engine.rs` `let parents = [...]`, microcode7 `exec.rs`
  `match number`). Two branches that each append collide: renumber one.
- **Library modules are compiled into the binary**
  (`langs/lib_python/modules/manifest.rs`, `include_str!`): rebuild after
  any library edit, and a binary built mid-experiment carries the
  experiment.
- Documented divergences not to chase: `//` truncates, `round()` rounds
  half away from zero, `divmod` floors differently (§4).

---

## 3. The verification sequence

The scripts are in `scripts/suite/` (see its README). From a worktree's
root, with its debug build:

```
RUSTFLAGS="-D warnings" cargo build
python3 scripts/kernel_independence.py          # must end "0 problem(s)"
python3 scripts/port_examples.py                # no git diff after
python3 scripts/lang_table.py                   # no git diff after
cp target/debug/lumen-lang /tmp/bin-<sha>       # count and long runs use a copy
python3 scripts/suite/scratchcheck.py . $(ls scratch/*/*.py | grep -v reader-tail/4.py)
python3 scripts/suite/scratchcheck.py . scratch/reader-tail/4.py     # alone, long cap
bash scripts/suite/test-debug.sh --lang python  # 384 of 384
bash scripts/suite/test-debug.sh --lang php     # 318 of 318
bash scripts/suite/test-debug.sh --lang lumen   # 522 of 522
LUMEN_ROOT=. python3 scripts/suite/count_run.py <rawdir> 900 /tmp/bin-<sha>
python3 scripts/suite/count.py <rawdir> <previous rawdir>
```

`scratchcheck.py` prints only mismatches and ends `checked N programs, M
mismatches`. `test-debug.sh` is `test.sh` on the debug binary with a 30 s
cut-off per program: under load a slow program shows as a timeout or
"differs from stream35" with empty output; rerun such a file alone on
stream35, stack8 and microcode7, and it counts if all three agree.
`count_run.py` runs each reference file once per kernel into
`<rawdir>/<test>.<kernel>.txt` and skips files already there, so it can
be restarted; `rerun_one.py` reruns one file with a longer cap.

**GitHub Actions runs the same three ways on every push** —
`build-and-test` (independence, the `-D warnings` build, ported examples,
all examples on all six kernels, on the release binary), `reference` (the
PHP row, kernel disagreements, regressions) and `scratch` (every scratch
program on both full kernels, comparing `.out` or the first `.err` line
exactly). A run takes about 75 minutes. Push a batch once it is verified
locally, then read the run rather than pushing again.

Never `cargo build --release` on a small machine: `stack8`'s release build
is one compile unit of about an hour and has killed containers. CI builds
the release binary itself.

---

## 4. What went wrong, so it is not done twice

- **The six kernels must print alike** on the examples; the four
  reference kernels read past every `ext.*` label, so a Python-only
  behaviour must sit behind an `ext.*` label, and a behaviour every
  language shares behind a core label all six read.
- **`//` truncates and `round` rounds half away from zero** for every
  language (the examples depend on it); CPython's flooring and
  half-to-even are documented divergences in `langs/README.md`.
- **Every roster line is one long line**, so a merge can take one side
  whole and drop the other side's labels without a conflict. After a
  merge, check each side's new labels are all still present.
- **"Pre-existing" must be checked against `main`, not the batch's own
  base.** A mismatch that exists on the integration branch before a
  worker's change may still be the batch's own doing; CI compares with
  the records on `main`.
- **A worker's gate claim must come from its own run.** One cited the
  coordinator's log for a gate it never ran.
- **Kill processes by id after checking `readlink /proc/<pid>/cwd`,
  never by pattern.** Workers run the same scripts; a pattern kill ended
  another worker's run.
- **The examples are load-sensitive**: rerun a timeout alone before
  believing it.
- **Keep whole logs**; a tail hides failures.
- **Copy the binary aside before a long run**: rebuilding under a
  running check makes it report nonsense.
- **Container rendering is a core label** (`system.collection.render`:
  `plain` or `representation`), read by all six kernels; do not mark
  literal-built containers quoted to reach the same end.
- **A map was a row of pairs searched from the front**; both kernels now
  hash, but a quadratic path elsewhere shows as a scratch timeout first.
- **The library's own bindings must not shadow a builtin word**
  (`program_bound` in stack8 `compile.rs`, `named_in_program` in
  microcode7 `build.rs`).
- **The whole run is made on a thread with a gigabyte of stack**
  (`src/main.rs`); the recursion limit needs it. Do not take it out.
- **`getattr`, `is`, `id` and `hash` reach values through cells** in both
  kernels (`Value::Bond`/`Binding`/`Collection` in stack8,
  `Value::Shared`/`Mutable` in microcode7); each builtin decides whether
  it wants the cell or the contents.
- **microcode7 parks a raised value while words stand in for it**
  (`got_away`): a road that meets such words must reclaim the parked
  value (`Err(Escape::Error(_)) if self.got_away.is_some()`), or a
  `StopIteration` becomes "special method returned an invalid value" and
  a stale value is reported by a later, unrelated call.
- **Scratch programs that write files write them in the working
  directory**: run checks from a tree root and make sure nothing new is
  left untracked (`@test`) before committing.

---

## 5. The machine

The work runs on an AWS instance the owner set up, driven through
`claude remote-control` and watched from the Claude app. There are no
inbound ports: administration is SSH tunnelled over AWS Systems Manager
(SSM).

| Item | Setting |
|---|---|
| Region | ap-southeast-1 (Singapore) |
| Instance | c8g.4xlarge (Graviton4, 16 vCPU, 32 GiB, arm64) |
| OS | Rocky Linux 10 (latest), arm64 |
| Disk | 80 GB EBS gp3 (default 3,000 IOPS / 125 MB/s) |
| Network | default VPC, public subnet, public IP for outbound only |
| Security group | inbound: none; outbound: all (SSM, GitHub, crates.io, the Claude API, dnf) |
| Instance role | an instance profile with `AmazonSSMManagedInstanceCore` only |
| Metadata | IMDSv2 required |

### Setting it up

1. **Launch** with the settings above and this user data, so the SSM
   agent is running before anything else (Rocky images do not ship it;
   without it there is no way in once port 22 is closed):

   ```bash
   #!/bin/bash
   dnf install -y https://s3.ap-southeast-1.amazonaws.com/amazon-ssm-ap-southeast-1/latest/linux_arm64/amazon-ssm-agent.rpm
   systemctl enable --now amazon-ssm-agent
   ```

   Check it shows as *Online* under Systems Manager → Fleet Manager
   before going on. As a fallback only, an inbound rule for port 22 from
   one address can be added for a few minutes and removed again.
2. **SSH server:** keys only. In `/etc/ssh/sshd_config`:
   `PasswordAuthentication no`, `PermitRootLogin no`. Add your public key
   to `~rocky/.ssh/authorized_keys` (through an SSM shell the first time:
   `aws ssm start-session --target <instance-id>`).
3. **The Mac:** AWS CLI v2, the Session Manager plugin, and credentials
   through IAM Identity Center (`aws sso login`) allowed to start a
   session on this one instance. In `~/.ssh/config`:

   ```
   Host i-*
       User rocky
       ProxyCommand aws ssm start-session --target %h --document-name AWS-StartSSHSession --parameters portNumber=%p --region ap-southeast-1
   ```

   Then `ssh i-<id>`, `scp`, `rsync` and `ssh -L` port forwarding all
   work through the tunnel. Raise the Session Manager idle timeout to 60
   minutes in its preferences if sessions drop.
4. **Toolchain on the instance:**

   ```bash
   sudo dnf groupinstall -y "Development Tools"
   sudo dnf install -y git tmux sysstat
   sudo systemctl enable --now sysstat   # load and memory history for `sar` (§2)
   sudo dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo
   sudo dnf install -y gh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y   # stable, as CI
   curl -LsSf https://astral.sh/uv/install.sh | sh
   uv python install 3.11          # probes' reference outputs were made with 3.11
   ```

   Keep `python3` for the scripts as it is, and call CPython 3.11
   (`uv run --python 3.11 python …`, or a `python3.11` on the path) when
   producing a probe's reference output, since newer versions word some
   errors differently. Set the time zone to UTC (`timedatectl set-timezone
   UTC`); the scripts assume it.
5. **GitHub:** a fine-grained token limited to this repository with
   contents and pull-request write access (`gh auth login`), `git config
   --global user.name/user.email`, and commit signing with an SSH key
   registered on the account as a *signing* key:

   ```bash
   git config --global gpg.format ssh
   git config --global user.signingkey ~/.ssh/id_ed25519.pub
   git config --global commit.gpgsign true
   ```
6. **Claude Code:** install and log in, clone the repository, and run
   `claude remote-control` in the repository folder inside `tmux` (or as
   a systemd service) so a dropped SSH session does not stop it. The
   session then shows in the Claude app.
7. **Check-ins:** the hourly check-in and pull-request events were wired
   to the old cloud session. Recreate them for the new session: a
   Routine if one can reach a Remote Control session, otherwise `/loop`
   in the session or a cron job that prompts it.
8. **Cost:** stop the instance when no work runs (the disk is kept and
   billed at about $6–8 a month); set an AWS budget alert; snapshot the
   disk before large changes. Resize only on the check-in
   recommendation's evidence (§2): stop, change the instance type,
   start, then restart `claude remote-control`.
9. **Security:** the instance holds a key that can push to this
   repository and nothing else. Keep it that way.
