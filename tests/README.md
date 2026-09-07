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
| `python/` | [python/cpython](https://github.com/python/cpython) `Lib/test`, the core-language files | `3b564385e4c9` (2026-09-07) | [python/LICENSE](python/LICENSE) (PSF License) |

A PHP test is a `.phpt` file: a `--FILE--` section to run and an `--EXPECT--`
(or `--EXPECTF--`, `--EXPECTREGEX--`) section to match. A CPython test is a
`unittest` module; none can run yet, and each stops at the first construct
the definition or a kernel does not know, which the report records.

The suites run on the two full kernels, stack8 and microcode7, which are
the ones that implement the `ext.` labels the languages need beyond the
core (see `langs/README.md`); the report scores each suite directory on
each kernel and lists any test the two disagree on. `php/basic` is largely out of reach by its nature: most of it tests
PHP's web behaviour, reading `$_POST`, `$_FILES`, `$_COOKIE` and
`$_SERVER`, which a kernel run from a command line has nothing to put in.
The suites are not part of `test.sh`: they measure distance, they do not gate. Run them with

```bash
python3 scripts/reference_tests.py            # both full kernels
python3 scripts/reference_tests.py --kernel microcode7
```
