# Reference test suites

Tests written by the languages' own projects, copied here unchanged so the
definitions in `langs/` can be measured against what the languages
actually do. `scripts/reference_tests.py` runs them and writes
[REPORT.md](REPORT.md): how many pass, why the rest do not, which reserved
words the definition spells, and which functions the suites call most that
it does not. That report is the list of what to implement next, in the
order the reference suites need it.

| Directory | Source | Commit | License |
|---|---|---|---|
| `php/lang`, `php/basic`, `php/func` | [php/php-src](https://github.com/php/php-src) `tests/lang`, `tests/basic`, `tests/func` | `8b0088a41de2` (2026-09-07) | [php/LICENSE](php/LICENSE) (The PHP License 3.01) |
| `python/` | [python/cpython](https://github.com/python/cpython) `Lib/test`, the core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_genericpath.py`, `test_posixpath.py`, `test_stat.py`, and the support data they read (`mathdata/`) | `8e6e75d9102e` (2026-09-30), tag `v3.14.8` | [python/LICENSE](python/LICENSE) (PSF License) |
| `python/test/` | [python/cpython](https://github.com/python/cpython) `Lib/test/__init__.py` and `Lib/test/support/{__init__,import_helper,threading_helper,os_helper,script_helper,socket_helper}.py`, and `Lib/test/test_genericpath.py` | `8e6e75d9102e` (2026-09-30), tag `v3.14.8` | [python/LICENSE](python/LICENSE) (PSF License) |
| `../langs/lib_python/modules/{os,stat,genericpath,posixpath,_collections_abc}.py` | [python/cpython](https://github.com/python/cpython) `Lib/` sources; unchanged beneath the two-line provenance header. Native POSIX and stat operations follow `Modules/posixmodule.c` and `Modules/_stat.c` at the same tag. | `8e6e75d9102e`, tag `v3.14.8` | [python/LICENSE](python/LICENSE) (PSF License) |

The `python/test/` package and its support, import, threading, OS, and script helpers
are also preserved byte for byte at that commit. The embedded runtime support
modules in `langs/lib_python/modules/test/` provide the interpreter adapters.

A PHP test is a `.phpt` file: a `--FILE--` section to run and an `--EXPECT--`
(or `--EXPECTF--`, `--EXPECTREGEX--`) section to match. A CPython test is a
`unittest` module. The suite measures released CPython 3.14.8 semantics;
unsupported behaviour remains visible as a failure or error.
The release repin covered 120 source/provenance entries: 73 test/support
files, 34 library source/adapter files, and 13 full scratch copies. SHA-256
verification matched 105 release bodies (72 tests/support files, 20 complete
library sources, and 13 scratch copies); thirteen documented partial runtime
adapters remain separate. Complete library sources are unchanged beneath
release provenance headers, with native bridges in `runtime_adapters/`.
The only removed files were the two copies of
`Lib/test/test_import/data/syntax_warnings.py`, which has no v3.14.8 counterpart.
Detailed working inventories and measurements stay in the ignored worker
scratch area rather than the repository.

The suites run on the two full kernels, stack8 and microcode7, which are
the ones that implement the `ext.` labels the languages need beyond the
core (see `langs/README.md`); the report scores each suite directory on
each kernel and lists any test the two disagree on. Much of `php/basic` tests PHP's web behaviour, reading `$_POST`,
`$_COOKIE` and `$_SERVER`. The runner gives each test the request its
`--GET--`, `--POST--`, `--COOKIE--` and `--ENV--` sections describe, the
way a web server would, so those tests run here as they run there. What
is still out of reach there is uploads, which nothing fills in yet.
The suites are not part of `test.sh`: they measure distance, they do not gate. Run them with

```bash
python3 scripts/reference_tests.py            # both full kernels
python3 scripts/reference_tests.py --kernel microcode7
```
