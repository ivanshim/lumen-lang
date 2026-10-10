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

## CPython 3.14.8 suite

The supported pins and directories come from
[`langs/python/versions.json`](../langs/python/versions.json). One release per
series is retained, in a window of two series; only 3.14 is registered today.

| Directory | Source | Release / tag | Commit / release date | License |
|---|---|---|---|---|
| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_super.py`, `test_csv.py`, `test_configparser.py`, `test_linecache.py`, `test_tokenize.py`, `test_abc.py`, `test_contextlib.py`, `test_ordered_dict.py`, `test_defaultdict.py`, `test_glob.py`, `test_timeit.py`, `test_datetime.py`, `test_getopt.py`, `test_optparse.py`, `test_wave.py`, `test_cmd.py`, `test_nturl2path.py`, `test_codeop.py`, and support data (`mathdata/`, `tokenizedata/`, `configdata/`, `audiodata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_raise.py`, `test_property.py`, `test_subclassinit.py` | `Lib/test/test_raise.py`, `Lib/test/test_property.py`, `Lib/test/test_subclassinit.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/` | [python/cpython](https://github.com/python/cpython) `Lib/timeit.py` and the library files it and its test import that the suite lacked: `Lib/getopt.py`, `Lib/gettext.py`, `Lib/linecache.py`, `Lib/wave.py`, `Lib/cmd.py`, `Lib/selectors.py`, `Lib/nturl2path.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test/` | `Lib/test/__init__.py`, `Lib/test/audiotests.py`, and `Lib/test/support/{__init__,import_helper,i18n_helper,threading_helper,os_helper,script_helper,pty_helper}.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_patma.py` | `Lib/test/test_patma.py` (unchanged; SHA-256 `ce4a802e1722fdd02ca6da58138a465a050edd360a150bbbba829586b3d15f55`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/{test_pickle,test_copyreg,test_pickletools}.py` and `python-3.14.8/test/{pickletester,picklecommon}.py` | `Lib/test`: pickle, copyreg, pickletools and pickle support classes, unchanged | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/codeop.py` | `Lib/codeop.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_super.py`, `test_csv.py`, `test_configparser.py`, `test_linecache.py`, `test_tokenize.py`, `test_abc.py`, `test_contextlib.py`, `test_ordered_dict.py`, `test_defaultdict.py`, `test_glob.py`, `test_timeit.py`, `test_datetime.py`, `test_getopt.py`, `test_optparse.py`, and support data (`mathdata/`, `tokenizedata/`, `configdata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_pathlib/` | `Lib/test/test_pathlib/`, all 12 files copied byte for byte (including `support/`); no separate data files in this release | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_raise.py`, `test_property.py` | `Lib/test/test_raise.py`, `Lib/test/test_property.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/zipfile/{__init__,_path/__init__,_path/glob}.py` | `Lib/zipfile/{__init__,_path/__init__,_path/glob}.py`, unchanged beneath a provenance/PSF header; required by pathlib’s ZIP-backed test support | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/urllib/{request,error,response}.py` | `Lib/urllib/{request,error,response}.py`, unchanged beneath a provenance/PSF header; imported by the pathlib test’s URI checks | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/runtime_adapters/native_test_support{,_os_helper}.py`: `_force_run`, POSIX `_rmtree` | `Lib/test/support/__init__.py` and `Lib/test/support/os_helper.py`: unchanged cleanup helpers, restoring permissions after a filesystem error | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/pathlib/{types,_local}.py` | `Lib/pathlib/{types,_local}.py`, unchanged beneath a provenance/PSF header | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_mimetypes.py` | `Lib/test/test_mimetypes.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/mime.types`, `mime.types2` | `Lib/test/mime.types`, `Lib/test/mime.types2`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_genericclass.py` | `Lib/test/test_genericclass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_structseq.py` | `Lib/test/test_structseq.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_flufl.py` | `Lib/test/test_flufl.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_tabnanny.py` | `Lib/test/test_tabnanny.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_strftime.py` | [python/cpython](https://github.com/python/cpython) `Lib/test/test_strftime.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_except_star.py` | `Lib/test/test_except_star.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_csv.py`, `test_configparser.py`, `test_linecache.py`, `test_tokenize.py`, `test_abc.py`, `test_contextlib.py`, `test_ordered_dict.py`, `test_defaultdict.py`, `test_glob.py`, `test_timeit.py`, `test_datetime.py`, `test_strptime.py`, `test_getopt.py`, `test_optparse.py`, and support data (`mathdata/`, `tokenizedata/`, `configdata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_csv.py`, `test_configparser.py`, `test_linecache.py`, `test_tokenize.py`, `test_abc.py`, `test_contextlib.py`, `test_ordered_dict.py`, `test_defaultdict.py`, `test_glob.py`, `test_timeit.py`, `test_datetime.py`, `test_getopt.py`, `test_optparse.py`, and support data (`mathdata/`, `tokenizedata/`, `configdata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_ucn.py` | `Lib/test/test_ucn.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_raise.py`, `test_property.py` | `Lib/test/test_raise.py`, `Lib/test/test_property.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_extcall.py` | [python/cpython](https://github.com/python/cpython) `Lib/test/test_extcall.py`, copied byte for byte; its `test.support.sortdict` helper is provided by the suite's support adapter | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_dynamic.py`, `test_longexp.py` | `Lib/test/test_dynamic.py`, `Lib/test/test_longexp.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_pkg.py` | `Lib/test/test_pkg.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_stringprep.py` | `Lib/test/test_stringprep.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/stringprep.py` | `Lib/stringprep.py`, copied byte for byte; `_unicodedata_320.py` is the runtime Unicode 3.2 database it reads | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_yield_from.py`, `test_generator_stop.py` | `Lib/test/test_yield_from.py`, `Lib/test/test_generator_stop.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_locale.py` | `Lib/test/test_locale.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/` | [python/cpython](https://github.com/python/cpython) `Lib/timeit.py` and the library files it and its test import that the suite lacked: `Lib/getopt.py`, `Lib/gettext.py`, `Lib/linecache.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

| `langs/lib_python/modules/` | [python/cpython](https://github.com/python/cpython) `Lib/timeit.py` and the library files it and its test import that the suite lacked: `Lib/getopt.py`, `Lib/gettext.py`, `Lib/linecache.py`, and the `Lib/urllib/error.py` and `Lib/urllib/response.py` that `test_ucn.py` imports | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `unicode-data/` | [unicode.org](https://www.unicode.org/) UCD files: `UnicodeData.txt`, `NameAliases.txt`, `NamedSequences.txt` (16.0.0) and `UnicodeData-3.2.0.txt` | 16.0.0 / 3.2.0 | 2024-02-02 / 2002-03 | [Unicode](../unicode-data/LICENSE.txt) |
| `python-3.14.8/test/` | `Lib/test/__init__.py` and `Lib/test/support/{__init__,import_helper,i18n_helper,threading_helper,os_helper,script_helper}.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_csv.py`, `test_configparser.py`, `test_linecache.py`, `test_tokenize.py`, `test_abc.py`, `test_contextlib.py`, `test_ordered_dict.py`, `test_defaultdict.py`, `test_glob.py`, `test_timeit.py`, `test_datetime.py`, `test_getopt.py`, `test_optparse.py`, and support data (`mathdata/`, `tokenizedata/`, `configdata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/codecs.py`, `encodings/*.py`, `stringprep.py` | `Lib/codecs.py`, 115 encoding modules and `Lib/stringprep.py`, unchanged beneath PSF provenance headers; low-level operations supplied by `_codecs.py` and `_codec_runtime.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_codecs.py` | `Lib/test/test_codecs.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_raise.py`, `test_property.py` | `Lib/test/test_raise.py`, `Lib/test/test_property.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_hash.py` | `Lib/test/test_hash.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_getpass.py` | `Lib/test/test_getpass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/getpass.py` | `Lib/getpass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_iterlen.py` | `Lib/test/test_iterlen.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_xml_dom_minicompat.py` | `Lib/test/test_xml_dom_minicompat.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_metaclass.py` | `Lib/test/test_metaclass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_sundry.py` | `Lib/test/test_sundry.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_pep646_syntax.py` | `Lib/test/test_pep646_syntax.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/` | [python/cpython](https://github.com/python/cpython) `Lib/timeit.py` and the library files it and its test import that the suite lacked: `Lib/getopt.py`, `Lib/gettext.py`, `Lib/linecache.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/xml/` | [python/cpython](https://github.com/python/cpython) `Lib/xml/__init__.py`, `Lib/xml/dom/__init__.py`, `Lib/xml/dom/domreg.py`, `Lib/xml/dom/minicompat.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_copyreg.py`, `python-3.14.8/test/{pickletester,picklecommon}.py` | `Lib/test/test_copyreg.py` and the pickle support classes it imports, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_rlcompleter.py`, `langs/lib_python/modules/rlcompleter.py` | CPython `Lib/test/test_rlcompleter.py`, `Lib/rlcompleter.py`, unchanged | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test/` | `Lib/test/__init__.py` and `Lib/test/support/{__init__,import_helper,i18n_helper,threading_helper,os_helper,script_helper}.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_tomllib/` | [python/cpython](https://github.com/python/cpython) `Lib/test/test_tomllib`: `__init__.py`, `__main__.py`, `burntsushi.py`, `test_data.py`, `test_error.py`, `test_misc.py` and the `data/` tree of `.toml`/`.json` fixtures | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/tomllib/` | [python/cpython](https://github.com/python/cpython) `Lib/tomllib`: `__init__.py`, `_parser.py`, `_re.py`, `_types.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_email/`, `langs/lib_python/modules/test/test_email/` | `Lib/test/test_email/`: every Python source and `data/` fixture, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/socket.py`: `getfqdn` function | `Lib/socket.py`: unchanged function in the native adapter | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/test/support/__init__.py`: `patch` | `Lib/test/support/__init__.py`: helper function copied unchanged into the existing runtime adapter | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/email/` | `Lib/email/`: every Python source, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/codecs.py`, `stringprep.py`, `encodings/` | `Lib/codecs.py`, `Lib/stringprep.py`, and all 115 `Lib/encodings/` wrapper bodies, copied byte for byte; reused from the signed codec port | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

| `python-3.14.8/test_patma.py` | `Lib/test/test_patma.py` (unchanged; SHA-256 `ce4a802e1722fdd02ca6da58138a465a050edd360a150bbbba829586b3d15f55`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/{test_pickle,test_copyreg,test_pickletools}.py` and `python-3.14.8/test/{pickletester,picklecommon}.py` | `Lib/test`: pickle, copyreg, pickletools and pickle support classes, unchanged | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/email/` | `Lib/email/`: every Python source, unchanged beneath a PSF provenance header | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/codecs.py`, `stringprep.py`, `encodings/` | `Lib/codecs.py`, `Lib/stringprep.py`, and all 115 `Lib/encodings/` wrapper bodies, unchanged beneath provenance headers; reused from the signed codec port | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

The `python-3.14.8/test/` package and its support, import, threading, OS, script, and
pseudo-terminal helpers are also preserved byte for byte at that commit. `Lib/test/datetimetester.py` is preserved, with its source header, in
`langs/lib_python/modules/test/datetimetester.py`. The embedded runtime support
modules in `langs/lib_python/modules/test/` provide the interpreter adapters.
The copyreg test's support (`test/pickletester.py`, `test/picklecommon.py`) and the
library modules it reaches (`pickletools.py`, `dbm/{__init__,dumb}.py`,
`http/cookies.py`, `test/pickletester.py`, `test/picklecommon.py`) are v3.14.8
bodies under PSF provenance headers.
The suite also holds `test_ordered_dict.py` and `test_defaultdict.py`,
copied byte for byte from `v3.14.8`; the support they import
(`test/mapping_tests.py` and `test/support/import_helper.py`) was already
present unchanged at that commit.

A PHP test is a `.phpt` file: a `--FILE--` section to run and an `--EXPECT--`
(or `--EXPECTF--`, `--EXPECTREGEX--`) section to match. A CPython test is a
`unittest` module. The suite measures released CPython 3.14.8 semantics;
unsupported behaviour remains visible as a failure or error.
The release repin covered 120 source/provenance entries: 73 test/support
files, 34 library source/adapter files, and 13 full scratch copies. SHA-256
verification matched 105 release bodies (72 tests/support files, 20 complete
library sources, and 13 scratch copies); thirteen documented partial runtime
adapters remain separate. Complete library sources now match the release byte for byte, with native
bridges in `runtime_adapters/`. Provenance is recorded here rather than
prepended to source copies.
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

The report runs every registered suite with its exact `--python` pin. To select
one suite, pass `--python 3.14`; `--binary target/debug/lumen-lang --no-build`
uses an existing debug build. Count tools accept the same `--python` option,
then `LUMEN_PYTHON`, then the newest registered release. Never edit a reference
file. A bugfix refresh uses `git mv` on its full-release directory and replaces
the table pin; it does not keep an older micro release. Add a provenance table
here for each newly registered series.

The embedded `langs/lib_python/modules/tempfile.py` is also a complete copy of
CPython v3.14.8 `Lib/tempfile.py`, unchanged byte for byte. Descriptor I/O is supplied by the runtime `_io` adapter and both kernels.
The pickle library sources `pickle.py`, `pickletools.py`, `_compat_pickle.py` and
`copyreg.py` in `langs/lib_python/modules/` preserve the v3.14.8 release files
byte for byte. The same is true for the pickle support modules
and their imports `dbm/{__init__,dumb}.py` and `http/{__init__,cookies}.py`.
The `_pickle` accelerator is absent; the upstream module selects its Python
implementation and the upstream tests select their accelerator cases accordingly.
`test_pathlib/` is a library test for profile 1.0 and is not included in the
counted core-language suite. Its package and support sources remain unchanged.

## Runtime fixture integrity audit (2026-10-02)

The annotation fixtures are test inputs, not runtime adapters. The earlier
re-pin inventory incorrectly classified all three as adapters. They are now
restored byte for byte from `v3.14.8 / 8e6e75d9102e`, without a header so that
upstream line numbers remain intact. This audit also restored the package
marker and `test/support/testcase.py`, including the complete exception and
floating-point assertion mixins and their original failure messages. Only
`testcase.py` has the usual one-line release provenance header.

All 22 tracked files under `langs/lib_python/modules/test/` were compared
by SHA-256 with the release. Nine already matched, five were restored, and
eight remain explicit interpreter adapters: seven host support helpers and
the partial `test_math` import bridge. These adapters are not unchanged
reference inputs. The `test_math` bridge supplies the parser, data paths and
`IsCloseTests` borrowed by `test_cmath`; it does not replace the reference
`tests/python/test_math.py`. No kernel or reference test was edited.

The scan covered all 107 tracked Python/data module files, including native
bridges and omitted support helpers. The wider directory contains complete
release copies, explicit adapters, and independent runtime implementations.
No other complete source copy differed. Files without a release `Lib`
counterpart are runtime modules, not purported source copies. Native bridges
were compared to their documented source module or C implementation. The
previous 120-entry re-pin inventory was recovered from repository history
for the inventory comparison; the new rows below record every audited file.

Hashes apply to file bodies after the single `# Source: CPython` provenance
line, where present. Adapter hashes deliberately differ; no claim of
byte-for-byte provenance is made for them. A dash means no same-path release
source exists. Full working measurements are in ignored `probe/`.

| File beneath `langs/lib_python/modules/` | Audit status | Local body SHA-256 after audit | Release SHA-256 |
|---|---|---|---|
| `_bisect.py` | runtime adapter / implementation | `7be8c113ef9145f3a20417a42cba270c5fa79b4bcf2a777ab705ba4022150826` | `f1cf7b85fc36b5da249813fc5ab97d9464f8cc1bc817f7146206fa2713e35999` |
| `_codec_idna.py` | runtime module; no Lib counterpart | `37a68a2b42648df2875c27ad62c43ebe29ec4c563425b2615daafb1dce2bf02f` | `—` |
| `_codec_names.py` | runtime module; no Lib counterpart | `e7e67d6d0c9469a30e9a7e4e66a1031a36822c948a029fb68a435aaedaf40773` | `—` |
| `_decimal.py` | runtime module; no Lib counterpart | `1b6401e46e7857037514520d81188bd06ddc3bcbdbb13fe39f2d6aa102b8a543` | `—` |
| `_heapq.py` | runtime adapter / implementation | `ef419577bd9fe9a78dab73bc5dd136c3caa188708390dd470a20680ca246acc4` | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` |
| `_namedtuple.py` | runtime adapter / implementation | `e958969735a74d30c2bfc44513cf80a52d53eca8afcd36b055c8e8bc1f266114` | `cb8367b8edd188662143ea2e3942d360c2deae51c72fd779ec898f5076d2c2b9` |
| `_operator.py` | runtime adapter / implementation | `7b8602c6cd09a27d22042ef7b3b292d1917f39cbc7d10fdbc4474a4cd856b163` | `5dd93483af61b7a2fb81ed3422c2270a69c606a49a9a01874176e496070015ae` |
| `_pylong.py` | runtime adapter / implementation | `98c670b97e52d4af88feec488a991b401f1bca9db0f06c0bb0c909f99822360c` | `3c82c3d1f8ed6058fc9b2c1362634d6fdfb8db9db207a980f702fedde5cb93ff` |
| `_string.py` | runtime module; no Lib counterpart | `7148f428f08029ca75af2df89d136f823a10cd784881632351b101dac539817d` | `—` |
| `_testcapi.py` | runtime module; no Lib counterpart | `9afe9bba125ba7bd61b0c3f95fffd6417d8fc662b865eec492ebf0530c3fbb8f` | `—` |
| `_testinternalcapi.py` | runtime module; no Lib counterpart | `1190ab49646d4b660caf831167daf3a291daa77b7de41411bd13cfcfc4006963` | `—` |
| `_testlimitedcapi.py` | runtime module; no Lib counterpart | `be7e729ffd3be51f19c9f00e1c26ecae693240219e4a845787f8be78df8849a6` | `—` |
| `_thread.py` | runtime module; no Lib counterpart | `e4b38cf018c66cbaa7a47d0944d3803c54b584361b4f1e9af0fcc8fa3cc14afe` | `—` |
| `abc.py` | runtime adapter / implementation | `603345f1fb09023bf4a7d6fd8110b30b9f597b75ebdaaba5a8c2231035c1ff9c` | `e558702a95cdce3febd289da021715d2b92bc43995b8a1bc58dfa1c3d8010287` |
| `annotationlib.py` | runtime adapter / implementation | `0e5cbe8e425fa84ece6d79ce7570fae01d0aa4b7a4caff8d972ed48684701cd7` | `1cce29393714b91e3f71571ffbedacc7f73f11ad26be10701debeb86ec059a32` |
| `array.py` | runtime module; no Lib counterpart | `ff86234099fa527fd6fbc1a2b91af824d55da715285a8780c3ff95a333f545ba` | `—` |
| `ast.py` | runtime adapter / implementation | `cd41975034508ed523abaf2492cafb83e94c4235fb6e23050c5b8c6383622aff` | `5ea9a796353544bea0d706153516f04ef3d92be305d15a73238403837d77e34c` |
| `bisect.py` | unchanged release copy | `f1cf7b85fc36b5da249813fc5ab97d9464f8cc1bc817f7146206fa2713e35999` | `f1cf7b85fc36b5da249813fc5ab97d9464f8cc1bc817f7146206fa2713e35999` |
| `builtins.py` | runtime module; no Lib counterpart | `c6b7e1eb4b4ad674e1e42fb716e69c97bb0c6ee4c8db6862e7615c9309d8a591` | `—` |
| `cmd.py` | unchanged release copy | `2412ce55bbaa0be0bb33e11e3a13735160194e674a7d88a262da708ee00e4d7f` | `2412ce55bbaa0be0bb33e11e3a13735160194e674a7d88a262da708ee00e4d7f` |
| `cmath.py` | runtime adapter / implementation | `c5a49f40a56b551c4ae6309a800f66e5d3f46a3036297754e93abb093d20494e` | `9687600b7e34cb6d2fefff71c32fa9f7d5646ffa64a38556f4f6e78f737161af` |
| `codecs.py` | runtime adapter / implementation | `4aa54c3ba26e60d235e73bbdb088af5f4a6f665eb5ad4b58263f9a7524128733` | `718f39b3ea68fe934214d789c5bce3005fa828040b03a72d2f715fc5e2647b7e` |
| `codeop.py` | unchanged release copy | `6777d19fa0f88c415d615c78e214ede057dd7507db8494744ca2c53fe9911a3a` | `6777d19fa0f88c415d615c78e214ede057dd7507db8494744ca2c53fe9911a3a` |
| `collections.py` | runtime adapter / implementation | `d28c865e493247540e55ea52caa18853bea753dd376fa4fe61655327ff5b4ca0` | `cb8367b8edd188662143ea2e3942d360c2deae51c72fd779ec898f5076d2c2b9` |
| `collections/abc.py` | runtime adapter / implementation | `982fca0cbabc5f0471367e174addb09dc0806b1742cb4137059078d86edb30cb` | `50b68b76687edc29824e5d735914b9c9ebb5145b663ba4ab7fce273283356a1c` |
| `contextlib.py` | runtime adapter / implementation | `77b2d8b4cb04a2ea75743c9c06c52a80e321d0a7f38a4a9b5c99df77c1190424` | `c1e0d67b2007de11ae93cd36cf6faf38d9ab32656a832d592a49325eec579f96` |
| `copy.py` | unchanged release copy | `15154fde290d6e65b5007905689a47a9a5a724822c5e06eec35d9ffbb2a3aa69` | `15154fde290d6e65b5007905689a47a9a5a724822c5e06eec35d9ffbb2a3aa69` |
| `copyreg.py` | unchanged release copy | `6376eb5722806396f5842997ac18add369ea9ac3ff4fcfe1460c41a088cac425` | `6376eb5722806396f5842997ac18add369ea9ac3ff4fcfe1460c41a088cac425` |
| `ctypes.py` | runtime adapter / implementation | `075bd2dbd11603cf7e6bfd5ef5cf7bc5e04df7418e66f100f00916a65efca7f6` | `349448c149c46962d6004808a214b4677267563204ec32cb6ef933effe0ee923` |
| `dataclasses.py` | runtime adapter / implementation | `c8cf26b317b606e905a14cd497518fda1a742ca2ce34b29482aad5e4f761a9d5` | `e8439c8b111856645a8b31e0dbbba532b494426399bad456079c0e5730ad322e` |
| `datetime.py` | runtime adapter / implementation | `0af428249328576d82d9b122a0d30fd6d15a22470886355d69aef460d76cacaf` | `742884b0c8e7dc69911858245db4f34d42e8ec0f769d580c570bd56032b0866c` |
| `decimal.py` | runtime adapter / implementation | `34c68ae6da3863025093c6f9a3f332ef1a54ebeb4bb1bab7c1533d6b69ff5f0b` | `af29fa99aa720a68cab5a6384c7beba4c8e9480a6c1d53a512efeeb5a7e5c781` |
| `dis.py` | runtime adapter / implementation | `374060342c94ef73b6e91a9644868d0460d8d35f19c7ac0fc1561f0b99670b00` | `17d4d9a9195c9ee5aa27a96f3e0871f9363fd501e3a64ef20cb1cc876d7ad553` |
| `doctest.py` | runtime adapter / implementation | `ad615d7193536046381d5d66eadab014bcd7edf7b1815add0cf7c50750fd7e03` | `3a5c30736974de19debbc9b3600461eef1943bd8b3b8afe3a634c05124b5dd5c` |
| `enum.py` | runtime adapter / implementation | `80ebe6d718e9601a77cc56c8a7a526b9e2938eebb0e6d308aded60ca4cabad11` | `fd23a7598fa1104ef892abcd4154d3627283e6361eb7a91cd11dc4a7b6fb3a93` |
| `errno.py` | runtime module; no Lib counterpart | `e9dac45709d8d26e173a71b7dc5b8507848e3729cfe7e591d8083992efe38d10` | `—` |
| `fnmatch.py` | unchanged release copy | `ce582bc266922e4c682e2a85d72095124430a6bf58716c6f2537103480eae742` | `ce582bc266922e4c682e2a85d72095124430a6bf58716c6f2537103480eae742` |
| `fractions.py` | runtime adapter / implementation | `503db9d4f73590da7bbbed2b1ff167348f24ebf256c5fbc373c5181e9f34f5ab` | `7a95f1c506c9ac4b2277df5f2bdd9d61cc67b520c45021a5a961939770221ef6` |
| `functools.py` | unchanged release copy | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` |
| `gc.py` | runtime module; no Lib counterpart | `a879e867e999268f591a51151fb5efe9d51a7046d9d155a6a5f58bfd76ef336f` | `—` |
| `glob.py` | unchanged release copy | `21d290be171e3d0643a444e4b58a2c4ee9bfe2c1474bb36ead043794bb769365` | `21d290be171e3d0643a444e4b58a2c4ee9bfe2c1474bb36ead043794bb769365` |
| `heapq.py` | unchanged release copy | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` |
| `inspect.py` | runtime adapter / implementation | `ef2deb2563813252ddd1fcc82e9347d22c4be4a0c90930a6690d80446d1f7f31` | `6daf297029d8971703b344de5cd485d71d4d935c72b85591493d8abc8512bdf7` |
| `io.py` | runtime adapter / implementation | `cf85cf25e6255e6793812652b1b2e8fce04946c90f88480c76b9ad7cb29ebba1` | `1b75584d4efcc612dc0db2ae98619408276585a106f0544879edb5e25b55430a` |
| `itertools.py` | runtime adapter / implementation | `43134d3fce836a09f53e800b9b5499e649232527540ded6aa5289682f5957704` | `11e8e8186a779a26ffd33bb5042b286bd08750a946c1398756225450d6bee6cc` |
| `json.py` | runtime adapter / implementation | `f5f9bf3a3ad059f0d80ac41dd60ba4fde5da461b8fc79d5f217243aa1fd324d4` | `2dd10f1bf4c9ea5478e589216805e7f279d0e4bce134a19efa297404fb87407d` |
| `keyword.py` | unchanged release copy | `18c2be738c04ad20ad375f6a71db34b3823c7f40b0340f5294d0e89f3c9b093b` | `18c2be738c04ad20ad375f6a71db34b3823c7f40b0340f5294d0e89f3c9b093b` |
| `locale.py` | runtime adapter / implementation | `cfd2a69a3010804d881928cce4fb4a73213fa21705d056ae9e490a2cae70efe2` | `f2e390ebde2bb52eabcf6d444a6a2e758749f3aa82c61427cbb053332fac56b2` |
| `marshal.py` | runtime module; no Lib counterpart | `627650ebff7594e5c03b1499cc8d7600787a1906aeaabb557f5f568d5347c76a` | `—` |
| `math.py` | runtime module; no Lib counterpart | `2d4a139f2ed29486305b3a3121e590ea552fe71e6ad7bc0c6a67fe4a432c3c96` | `—` |
| `mimetypes.py` | unchanged release copy | `1307cd9736918e9e1dc1f8fe15539eef97c52769db23bf0fc207b5e5ed8c57f2` | `1307cd9736918e9e1dc1f8fe15539eef97c52769db23bf0fc207b5e5ed8c57f2` |
| `numbers.py` | runtime adapter / implementation | `c3a6aa26be4af50771bf76b7bb556c40d95a3432ab8c11b9072c356c9effd4ce` | `e5e73beba4a7674bd5e9a881a202d403a0c3e2b59af4181699155ea4827a7561` |
| `operator.py` | unchanged release copy | `a9f9910965c31f841caad0447ee47299ac539a371422aefe200c86c60f3b4697` | `a9f9910965c31f841caad0447ee47299ac539a371422aefe200c86c60f3b4697` |
| `os.py` | runtime adapter / implementation | `a2d7473b7b5a05e296b58980790ccfc38feeab6a93039187725e3c18517ab827` | `976bdb3e24925f2fb3ce43a4d788de6dcbb88a7cc8604c60ae690fbdd6507546` |
| `os/path.py` | runtime adapter / implementation | `bf2ce27e89674f859c937b52bac276b34002f1b34c2c1f7a11c39ede20235275` | `8a5bc2b6674e76efe0f507b873a41d746beba466a191153ed18a7580ef6e08e1` |
| `pdb.py` | runtime adapter / implementation | `1a058afea49ceca837b238c1bc0f6aa5cc04119ff283f7935b198946b7023d50` | `50d5e0f9dab1f4e2d114c6e6d62577d8c76018c8732443a48bf783671179317f` |
| `pickle.py` | runtime adapter / implementation | `5347a713d3192ff3044178d5e04e9e5dc0701d17cfc477d6fefac03293c8c16f` | `8e0aa6be0b8232bf352f086696da1c6e1d55e3eb27f0af5f472f466d0dbd9848` |
| `platform.py` | runtime adapter / implementation | `061d8bc521d94f27fca5064a9665a0198292902c2be200783e6b49d9cfab3c45` | `240eba74565cd0dc4f99f182f7afc32bbea0593016ddb34165b529b5bcb6be0a` |
| `posixpath.py` | runtime adapter / implementation | `5545d954ca2362c9643e439abe5ed18d9da7a04ab3923e1cabbff807caeece10` | `8a5bc2b6674e76efe0f507b873a41d746beba466a191153ed18a7580ef6e08e1` |
| `pprint.py` | runtime adapter / implementation | `352a0d6b5353b6328c6b1860dc651694b0d29ce26e1d3a5581de6ec50affe169` | `0b807d4a62c4292ead5b64e1b455f98b0ba225667ff3ee74bd7e0b873620402d` |
| `pty.py` | runtime adapter / implementation | `10736d1b718cddab726c2bdd0d0a97065819c546625cddc1780eb5d69cca6176` | `655814df6302412a3991305d05a301e0a2514d1363b7465a30898513a53bddfc` |
| `random.py` | runtime adapter / implementation | `c981fd97a1e5c8f91001a30c4fb17da9065437af7953b172456139c36e2792b6` | `cf8e72f887d838e273c274b79758b2bf8630a0927112b8876fa3a9e4b9b1756b` |
| `re.py` | runtime adapter / implementation | `a803e6a068b211d46f3188662b366dc69984ac61bda074c502e82fbadf8824af` | `741a9de729ed8207bfa19db990f8826f1bf3661f33d0970a80c08cd1338ebc35` |
| `reprlib.py` | unchanged release copy | `b04872e10d76252e44eae50cc785eb0fe478491afc4d7f0825a93f82b2eaba5f` | `b04872e10d76252e44eae50cc785eb0fe478491afc4d7f0825a93f82b2eaba5f` |
| `runtime_adapters/copy.py` | runtime adapter / implementation | `c0a733b4c73e2c77f4b8b769717b771a44758ee169d27a99c755ebf2d16eacc4` | `15154fde290d6e65b5007905689a47a9a5a724822c5e06eec35d9ffbb2a3aa69` |
| `runtime_adapters/copyreg.py` | runtime adapter / implementation | `bc1e513213cb47877a24e1aa3f8b34c43e85332804558618fa8680739b511ce1` | `6376eb5722806396f5842997ac18add369ea9ac3ff4fcfe1460c41a088cac425` |
| `runtime_adapters/functools.py` | runtime adapter / implementation | `1fd5b65b99cc8a2e6217822bce27baf6a833acc2a49cfbf4c942d9e1dd6976e4` | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` |
| `runtime_adapters/heapq.py` | runtime adapter / implementation | `6a33f1c1a941416c31a4c9375a3a4bcb3fd3c06fab0dc3961d48ec27b9ddd8b0` | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` |
| `runtime_adapters/operator.py` | runtime adapter / implementation | `cc9b0d8240d453606afb7556e3e1004196df2652822f5ce5d0d0e1bd57f1a95e` | `a9f9910965c31f841caad0447ee47299ac539a371422aefe200c86c60f3b4697` |
| `select.py` | runtime module; no Lib counterpart | `394bda868825731bcb4000080590dc62fc98c14e27652fe1b23aa8d9f635b063` | `—` |
| `selectors.py` | unchanged release copy | `4b8a60cfbb619d080f87dffe1578372e56e8c0ac826f043224a154cf7b77606d` | `4b8a60cfbb619d080f87dffe1578372e56e8c0ac826f043224a154cf7b77606d` |
| `shlex.py` | unchanged release copy | `aeda1c54188363d907654a19d338e5961416ebec2e2f506afab19be61ada8120` | `aeda1c54188363d907654a19d338e5961416ebec2e2f506afab19be61ada8120` |
| `shutil.py` | runtime adapter / implementation | `adf9e03edd25763490ee7d77f27fc8f8bdebf4bd4d0b2c7226699eb07bda968a` | `28a5df6415bf1a7ab36cbd544d6d35ae0f056a198e3872a786dc4e156d5fcd7d` |
| `signal.py` | runtime adapter / implementation | `827449d3ecf3086232ec54bbe07e16ea5987f08ff70be9ff8f27e0ace8ec4214` | `0363c964c90ac0b3e515de5749205e6e6454051a1211058375d84d91eab6071a` |
| `statistics.py` | runtime adapter / implementation | `0d3d742eba317f6d8efda263bf040df59b6d051fc511d5ccf7c934ba9ae289b2` | `b7558385a2005432c0ae74a222866316bf4f9c3ed448555659e7014b1077173a` |
| `string.py` | runtime adapter / implementation | `87cf93090f882c1dc39ad9b2a9cd0ef9770415936b0aec421617a2cc6c5e9540` | `8c9751e6ae9776b4e94c1c9850b53c0d24e8407196901aeaebffbb1a11a64e05` |
| `struct.py` | runtime adapter / implementation | `400595da579c406bbe6f5a3dec52d7d728910d4927e878acad68678c0cc8f0d0` | `50c14e55f94957804c1e45e975fedbe8e86f7e1dbb745b150370ff681155370a` |
| `subprocess.py` | runtime adapter / implementation | `b3947dd75e8d4b84a7e4e84e522c1a5a13f7880571b4f820d3da5f3a2e8e8b2e` | `7fa4af83422024a19e64a80937f6c8094244c3397372a495b348dfbe44a5dd72` |
| `sys.py` | runtime module; no Lib counterpart | `90019bb1d77e139ef774cc82b3d01e5206435529c869732c137bd8777b5eb443` | `—` |
| `tabnanny.py` | unchanged release copy | `26d014ee01ce9a469d262b3f1f6a8be341b47ac45eba57472fbd7f96e57a05a2` | `26d014ee01ce9a469d262b3f1f6a8be341b47ac45eba57472fbd7f96e57a05a2` |
| `tempfile.py` | runtime adapter / implementation | `bc171d211898ecf33be9df1c08a48ce09e3b9e0852e13447b4f508e8d9e1fb63` | `3714372b63f6bc05a5273425d446f6ad3a2112a659c71c321bb4575fd082d634` |
| `test/__init__.py` | restored release copy | `836cdb388117cf81e78d9fa2a141cca1b14b0179733322e710067749a1b16fe9` | `836cdb388117cf81e78d9fa2a141cca1b14b0179733322e710067749a1b16fe9` |
| `test/audiotests.py` | unchanged release copy | `963c93fafcb826c1f368cf3c033605cc8b196ccc18d9fe2d364a8ce34372882a` | `963c93fafcb826c1f368cf3c033605cc8b196ccc18d9fe2d364a8ce34372882a` |
| `test/list_tests.py` | unchanged release copy | `41c15b8e00b1e1474e519567b1124d3c5fd7fef975a200a5be276fd2ec8eaaf7` | `41c15b8e00b1e1474e519567b1124d3c5fd7fef975a200a5be276fd2ec8eaaf7` |
| `test/mapping_tests.py` | unchanged release copy | `864213137e72ec2ea142c5c4596755c52b21b58095223884b2432e73469671fb` | `864213137e72ec2ea142c5c4596755c52b21b58095223884b2432e73469671fb` |
| `test/mathdata/cmath_testcases.txt` | unchanged release copy | `6a41e9bf349e4de95f44133eb3fd4804d2c373436fc48a26f94df39f26aeca04` | `6a41e9bf349e4de95f44133eb3fd4804d2c373436fc48a26f94df39f26aeca04` |
| `test/mathdata/math_testcases.txt` | unchanged release copy | `bb3a7ccb8adc60317861bf79402f9a5ee0f1e35f81010f694effb86d78e5d985` | `bb3a7ccb8adc60317861bf79402f9a5ee0f1e35f81010f694effb86d78e5d985` |
| `test/seq_tests.py` | unchanged release copy | `8d3af229c2b6d204410badfaf530e902aa451ae10500662049c7941709d3a591` | `8d3af229c2b6d204410badfaf530e902aa451ae10500662049c7941709d3a591` |
| `test/string_tests.py` | unchanged release copy | `da69e072f4b439480d1fd434973e0fe8e969abb8d5150cadf6580106038ee04b` | `da69e072f4b439480d1fd434973e0fe8e969abb8d5150cadf6580106038ee04b` |
| `test/support/__init__.py` | runtime adapter / implementation | `b0063ce742f8f7017f9304352e06cf903c8c316a1eb4590e4972c01e362594d0` | `450e2f52f881c64b617aa07c77535e3125d806038e8015596105589adcf358bc` |
| `test/support/import_helper.py` | runtime adapter / implementation | `32901db492023178e4ced1454259c6c5df20840d4cc9034dfe968ca8ba20a42b` | `028ab3a396d3b5f57e12e755351458dc26604de96540fec0bf6d9904b3ca8f11` |
| `test/support/isolation.py` | runtime adapter / implementation | `5ef57aea7ddde7673c9f495c11d1ae88d3c2059b6ba9e3dabf645cbbf7a7564b` | `3d0302f6c7fb0cf4bdca890879dc111950b66fadfb4c3a29516f9f128c5ad846` |
| `test/support/numbers.py` | unchanged release copy | `fd9c8f35ef65c32612599a89a3ff8fe320268bd139a1c1a773cdfdd44096202c` | `fd9c8f35ef65c32612599a89a3ff8fe320268bd139a1c1a773cdfdd44096202c` |
| `test/support/os_helper.py` | runtime adapter / implementation | `e89dff9daab547f11b11f802a1c2d29dd18c864ea0d8f6babdd99deee8cf4068` | `d42f1738a54a378b5d1c4c61f6ab6e7eb12735aff6785a2ec14e28240843b66f` |
| `test/support/pty_helper.py` | unchanged release copy | `5b25070c482572da991fb5a9152368b8b8c1f57ee31dfec954409ebd7073dfe3` | `5b25070c482572da991fb5a9152368b8b8c1f57ee31dfec954409ebd7073dfe3` |
| `test/support/script_helper.py` | runtime adapter / implementation | `4ce0a8771b3aaf48ffc2fd01a00b423c24b281f6f849a38df65603344b6a7e55` | `d94c1502c2b7a3f1e57deb1c8913e1890049c73979261f205fc2f7e96fc03677` |
| `test/support/testcase.py` | restored release copy | `69683bb4a66f7abfb91c5726b0e6c2434a2e1bd2d7ddda5b88b602b1577be12c` | `69683bb4a66f7abfb91c5726b0e6c2434a2e1bd2d7ddda5b88b602b1577be12c` |
| `test/support/threading_helper.py` | runtime adapter / implementation | `32b013b39f834663bd1f253ca11cc18e99a5cdb558178e1a35b8ada372442f76` | `93976321a0592dda85d769089d443f7ce6f0742911852d3d875de5f03a8d7471` |
| `test/support/warnings_helper.py` | runtime adapter / implementation | `065324aee43a5c679cf698c0155583b064960153f7f37ae83589b01fdfbf2807` | `90576deca30ac0986cb2bdd0dc715363dfe90ed770382991d0674e6533c1ca46` |
| `test/test_iter.py` | unchanged release copy | `2255bb4dac0165fd5f9bcc56112ec938722d425b819099749efdd24b9193887e` | `2255bb4dac0165fd5f9bcc56112ec938722d425b819099749efdd24b9193887e` |
| `test/test_math.py` | runtime adapter / implementation | `1f352896629e30e7e6f6d2efab100e559a6864d003dab0638cd9db3c2476e711` | `17a9b4e60bcf3e0ac185d0b4f4d97ab1ea91c4f94069c3032ba56d7994930b03` |
| `test/typinganndata/__init__.py` | unchanged release copy | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `test/typinganndata/ann_module.py` | restored release copy | `54ac1c9629d3e0ff311ef13b019604dc8d7b17a1dafd3ff113c9dba095717ee2` | `54ac1c9629d3e0ff311ef13b019604dc8d7b17a1dafd3ff113c9dba095717ee2` |
| `test/typinganndata/ann_module2.py` | restored release copy | `2f1214af1113c659b37ff02aa9727f3341812e066c82524c471e4325bcde6f72` | `2f1214af1113c659b37ff02aa9727f3341812e066c82524c471e4325bcde6f72` |
| `test/typinganndata/ann_module3.py` | restored release copy | `c72c7dfa54f5af1bb9ad263964adf130597666ae1e5cd125f5a435b565d6c15f` | `c72c7dfa54f5af1bb9ad263964adf130597666ae1e5cd125f5a435b565d6c15f` |
| `textwrap.py` | unchanged release copy | `7f05d89c983f5aa8cefcff707a9cc561ed60c51fc8a68e2e3ab61cc6dfa84544` | `7f05d89c983f5aa8cefcff707a9cc561ed60c51fc8a68e2e3ab61cc6dfa84544` |
| `threading.py` | runtime adapter / implementation | `de45da3f7172546ba2d7f9d02279b743e9bfa448cb1ed1aa18c78a98cf29ba58` | `5323909624ec2165e70b6d31333e4191b63d383d2dc5a7d7d516a3475ea2b7e3` |
| `time.py` | runtime module; no Lib counterpart | `476a3db6257675a4489dbe0590815c5e21db4cc275531d1f09037cfb777d4e73` | `—` |
| `traceback.py` | runtime adapter / implementation | `b8969448d944948de2433d92a9c2ee1fad93aafb871e7c04364741ea5559a704` | `a8fbeea470aa4e29691560de889de136570bead76d9c8a2b6a891849c047a3b3` |
| `types.py` | runtime adapter / implementation | `36f90be33b6a1bf0e45b038426343a37c90a60bdcf016b4239c130af1d0459e3` | `8c54d3d5ffc1d1204237e6c69b25c27c7b05b483128f185eeed9ba7ef2229ac2` |
| `typing.py` | runtime adapter / implementation | `8cab6db1cd142fccfb5ad262197adeb9f5862795c2f3afa5ac1625e7fa569f2b` | `de569368c2c4958b7aaddbe755056860a89b1d313f69d437177864f37cb2503b` |
| `unicodedata.py` | runtime module; no Lib counterpart | `776531547872ac6a0dc7de238c65b785c8be4fe9a1cf0ae66b2ca662a282c273` | `—` |
| `uuid.py` | unchanged release copy (byte for byte) | `83529261c33ec06057420d8142fa9f8d35a50232bc011ff33d9f442d056cbd9e` | `83529261c33ec06057420d8142fa9f8d35a50232bc011ff33d9f442d056cbd9e` |
| `unittest.py` | runtime adapter / implementation | `59844316763609ebd8d74409e324c207e9305f76a973b5e32efbebffffad554c` | `2698f2daf7a02609a6825b89cd459ebaa283dec2403c7b846315a0701bc74a0d` |
| `unittest/mock.py` | runtime adapter / implementation | `1cd52e4f3568e9bf3e081f24129fa5fba53c517fae4ab6de4ce6b0d98444a929` | `856148cdc93943b4ff948276e94561db3d6c44ddecacf53c3d25eb7dc46a108d` |
| `wave.py` | unchanged release copy | `9b2249d6e3d0d1b2cbc183f59ca301cdfeb34f1b022f8712bf0318c2618a4edd` | `9b2249d6e3d0d1b2cbc183f59ca301cdfeb34f1b022f8712bf0318c2618a4edd` |
| `warnings.py` | runtime adapter / implementation | `ee0bf5d9c33be43a8deec328c8970c4570ef75c9496979a7abeed64aafbfad6b` | `142225786de63c593f1c9abdacf5b4fc0b05dd847f6bed0ebb4b4aa2d4d93b02` |
| `weakref.py` | runtime adapter / implementation | `e080fe0eeefbc2018cfe0a90f71d159599c4b225e4c319289f3896dec7383728` | `5e5f727a19a858cb4c56dbaf3e0a138ded02fb954a9a58a840a0764216ae9522` |
| `xml/__init__.py` | unchanged release copy | `34296f728e7fe68cccb97a9f6edbf3bf3a686f44044c744fe85f207a92ed4811` | `34296f728e7fe68cccb97a9f6edbf3bf3a686f44044c744fe85f207a92ed4811` |
| `xml/dom/__init__.py` | unchanged release copy | `e7139ed583a7f60fbbc750044df4f1e3655371d8b8c4f80a43bff4aa3ba97857` | `e7139ed583a7f60fbbc750044df4f1e3655371d8b8c4f80a43bff4aa3ba97857` |
| `xml/dom/domreg.py` | unchanged release copy | `826b02a803930834b96b1086cbee7db1d21c684f65dd3073706dc7bb5ba1a3e8` | `826b02a803930834b96b1086cbee7db1d21c684f65dd3073706dc7bb5ba1a3e8` |
| `xml/dom/minicompat.py` | unchanged release copy | `42974c4c67803dfe80b016ff8aeea0d1e5c751703ab3aec5be765f4e534367be` | `42974c4c67803dfe80b016ff8aeea0d1e5c751703ab3aec5be765f4e534367be` |

### Reference measurements

Lambda before/after runs used this worktree’s warning-clean debug builds.
Both kernels produced the same results below. `∅` means no progress line:
the module failed during import before unittest ran.

| Reference file | Before pass/ran | After pass/ran |
|---|---|---|
| `test_grammar` | 75/75 | 0/0 |
| `test_opcodes` | 8/8 | 0/0 |
| `test_builtin` | 108/133 | 108/133 |
| `test_cmath` | 32/33 | 32/33 |
| `test_complex` | 37/38 | 37/38 |
| `test_float` | 50/54 | 50/54 |

Progress lines (identical on stack8 and microcode7):

`test_grammar` before / after:

```text
...........................................................................
∅
```

`test_opcodes` before / after:

```text
........
∅
```

`test_builtin` before / after:

```text
.................EE..................E...s..............................ss.........s.......s....sssEssssss......s.ss.s.....E..E.....E
.................EE..................E...s..............................ss.........s.......s....sssEssssss......s.ss.s.....E..E.....E
```

`test_cmath` before / after:

```text
..............s..................
..............s..................
```

`test_complex` before / after:

```text
..s...................................
..s...................................
```

`test_float` before / after:

```text
s............s.....................s................s.
s............s.....................s................s.
```


The restored `ann_module` imports `types.new_class`, which the runtime
does not provide. `ann_module2` independently imports `typing.no_type_check`,
which is also absent. CPython 3.11 imports all three release fixtures; both
kernels now report the same catchable import errors for the first two.
`ann_module3` imports on both kernels, and all three erroneous annotation
cases raise the same `NameError` messages as CPython 3.11.

Restoration exposes 83 formerly reported passes on each kernel that depended
on altered inputs. Grammar and opcodes have no after progress line, rather
than individual dots turning into errors. The four assertion-helper consumers
retain their exact progress lines and counts. The pass-preservation target
cannot be met by an integrity-only restoration while those runtime features
are absent. Scratch `reader-tail/3` and `reader-tail/7` therefore keep their
previous records; replacing them with import errors would hide the missing
test executions. No scratch record was moved.

## Batch21d source and async audit

The integration compared all tracked reference files and same-path library
copies against CPython `v3.14.8`: 530 files match byte for byte. Thirty existing
explicit runtime adapters differ by design; reference inputs have no mismatches.
Sixteen complete library/support copies lost their added provenance line so
the entire files, including line numbers, now match the release.

The runtime has one `test.support.subTests` implementation. Its async wrapper
awaits coroutine methods, including partials. The inspect adapter recognises
partial/partialmethod wrappers and coroutine markers. Deliberately failing
async subtests and async partial subtests are checked on both full kernels.

Partial native and support interfaces with CPython-derived sections keep their
runtime implementations under `langs/lib_python/modules/runtime_adapters/`.
The `replacements` entries in `manifest-layout.json` bind those implementations
while retaining byte-for-byte v3.14.8 library files at the upstream paths.
An existing `adapters` overlay still follows a replacement body. The ten source
moves preserve the registered interpreter source bytes, including the single
async-aware `test.support.subTests` binding and coroutine recognition.
The email charset dependency reuses the signed codec port at
`ecee70ec399ce9338ba6267d06aab481206c43f5`. The released sources in the
provenance table stay unchanged. `_codecs.py`, `_codec_runtime.py`,
`unicodedata.py`, and the multibyte family modules are native runtime adapters.
Mapping data provenance is recorded in `langs/lib_python/data/multibyte/README.md`
and `langs/lib_python/data/unicode/README.md`.

## Batch21g source fidelity

All complete library copies introduced in this batch match CPython `v3.14.8`
byte for byte, including original line numbers. Release provenance is retained
in the table above; runtime changes remain in kernels and explicit adapters.

## Codec release source integrity
The codec adapter now lives in `_codec_runtime.py`; `codecs.py` is the complete
release source. The following hashes cover unchanged bodies beneath the single
provenance header. CJK wrappers retain their native dependencies and fail
honestly when those extensions are unavailable.
| File | Release body SHA-256 |
|---|---|
| `codecs.py` | `718f39b3ea68fe934214d789c5bce3005fa828040b03a72d2f715fc5e2647b7e` |
| `encodings/ascii.py` | `578aa1173f7cc60dad2895071287fe6182bd14787b3fbf47a6c7983dfe3675e3` |
| `encodings/base64_codec.py` | `cf9ac7a464f541492486241d1b4bf33e37b45c6499275cc4d69c5a8e564e5976` |
| `encodings/big5.py` | `98fac6f86a20dd05da197e2058176ebfd47edee7074c3248f5f48fe0fb672d7c` |
| `encodings/big5hkscs.py` | `21d051a00fb5c6a86ba187e0c50e811d659ce00991fd5f5b408f71ebb2ef0f16` |
| `encodings/bz2_codec.py` | `1181a2a89102a2b1d2b2f1f4473236d5d1ececdd0be8fdaa498a3dbe21a185ab` |
| `encodings/charmap.py` | `1b8b5fdb36ce3becc62a6115ed904a17083949ec8aaef5a80f7078cec232f43b` |
| `encodings/cp037.py` | `fda6ca994d710e4e0c760e0204c29a4273fc0f14ebe3169306d2eb54c9953f58` |
| `encodings/cp1006.py` | `eaded38b427841bdf280e878f1e26da506e743eaa9429075332af60cce429473` |
| `encodings/cp1026.py` | `f5227237dd7ce5005b16a8e4d8342f0d193193c878e3cf35b9305d22b3b1aaf9` |
| `encodings/cp1125.py` | `f84c7d30ce222e6a50cff1a4c9737173411da108cbd2c9bb57c854480103c470` |
| `encodings/cp1140.py` | `3379d78b244aa905ffe1171a968caaf41b9a0154d1ddc76c05a2abaca2b289fd` |
| `encodings/cp1250.py` | `ebcec1adf9167863fb0bab29708c546300c80a77ef07838c9e0437a59e265970` |
| `encodings/cp1251.py` | `d57f8cfa34494c5acb6692ddb31f616ae2dd89a075d2af6d36b0b7ec2ffe7af1` |
| `encodings/cp1252.py` | `19aa5bee667f5fb387924a813aec9fa1dda47769d09e8483a748bdb202be6a84` |
| `encodings/cp1253.py` | `8c27696dcfb6894b378869bc89f113703fbd1e9b13a83934463d5999b055d1e8` |
| `encodings/cp1254.py` | `06517ec2f74f1c6562d0a1a500c48ba43f2e6e9d0c3d28356d747f274f1a4c8d` |
| `encodings/cp1255.py` | `54a1b5087578fa78e5bdd0afa6a9e80e8c5467c1e4226cf6e586cfe7a674a653` |
| `encodings/cp1256.py` | `ad3768ac2fef2a646b3301c20af705f4d4a1544f22fa8a84241bada27ab84133` |
| `encodings/cp1257.py` | `d9149d2925b3f719809ef2297e541461079f15c658af207a3e498be314ab2c6b` |
| `encodings/cp1258.py` | `672e05b51952a82c8dbd5603769195fcedf565e457bb86c0d5bae04955d04630` |
| `encodings/cp424.py` | `30414c2186ea0802bbf3db034122ddec1f8a10061b97c50871e14b74ee36d0ca` |
| `encodings/cp437.py` | `5c2a5015cd36cf7f561269f33dec4c323093d3d88b0673969accdabdcb9ce2cb` |
| `encodings/cp500.py` | `630f503f9110d98ea3e1529f2f965ebc275a2f78d3de47f8e9b69d35589d764b` |
| `encodings/cp720.py` | `395496001271b92efe5df07fc0ae7c3410d1dd2bdfebbd3e4d8e806c8166beb0` |
| `encodings/cp737.py` | `be3ca1785a3970ec62310710eaf7de82932181b04d06fe4528f8adaba9fb8c4b` |
| `encodings/cp775.py` | `e0dba85b99329d7f16907e620adada06be5216abcb964406c827b569b2cf1aeb` |
| `encodings/cp850.py` | `257e29f235e2a8790dd68cee45668776648bab809ce8584f893cdd8fd007993c` |
| `encodings/cp852.py` | `cc6faaa9dc4a933127da0aaacd1dc7a44c09266051af56bfe3215ff228636b6b` |
| `encodings/cp855.py` | `7b25c61c9e8c47b218d3fbb801541a2861926ac712843d2113fff90e2074f5ba` |
| `encodings/cp856.py` | `2e52ec5cb1eafa6739b5569b0b98ee89df5f7358b84ccdc8da64e86f017d359f` |
| `encodings/cp857.py` | `8d1b769058bfccdb3c6c70c49a104f5081a2fcc9fad68f7b5eb3e4f67f0b33da` |
| `encodings/cp858.py` | `a24930c4a6ad0ff66dde9a69f2027e4b92c2c9c61dcda2992e940654c606577b` |
| `encodings/cp860.py` | `2dfae7e31d3d9aa3013cff44a4d7ad842f257ac63765a9998436701b629cd86a` |
| `encodings/cp861.py` | `701930d77a2177497586e99bc3fe60f2d4beffb645608f167c76874a72ff405e` |
| `encodings/cp862.py` | `15a2844b6ed9544c6400cf7299b42d0c2bef93c9bee70a9e89f66b8610ad6d6d` |
| `encodings/cp863.py` | `a3d57f61fce1b98fc81ea8e4ebebaf402fae40bbcdd35d4b8297b9bb49a79aa2` |
| `encodings/cp864.py` | `15ad8f1fdfdd842c7522241372e7eddda7df687e815692a89157c5f256f21a08` |
| `encodings/cp865.py` | `bdbaded987242ed2a8de7133ec2f61ddcc1c2e9de27816ab7cd0a4c678a3a907` |
| `encodings/cp866.py` | `9efcc8e85bbd1687272a0991f6d0429a4c06679db2d114b2ac95db27a70f9d13` |
| `encodings/cp869.py` | `52582d9fb769b24eac7154f18d7dae856588297d6da98f37fb5efd8da883826d` |
| `encodings/cp874.py` | `fe4752fa2e65741e08a563a31ff914fe71068942ce9c6f4070b1dfd7b25e5e7f` |
| `encodings/cp875.py` | `2fe72632015db2cba2bb4367055551da6fe22051b96d170c7b96fa271c46b257` |
| `encodings/cp932.py` | `99748e28113d2d49f5d666b49b78accd2c6e10a7852f7dd6dece9b5b71aa83c4` |
| `encodings/cp949.py` | `950a7d29467ce0590b4a1137830d43d88d8f20e4035dcaaa8b2a5c3c3f1de962` |
| `encodings/cp950.py` | `27811178b450731fc955b1247656a605d04e5ee98e0d585e4596b94b703a27f6` |
| `encodings/euc_jis_2004.py` | `9fa426cd9f17629f6320700ed18baa94839304cf1bcabbee7edb501747dc055d` |
| `encodings/euc_jisx0213.py` | `e28315910da20218dae8b7d5becd81de1e283dfd8b0415a4980d67065de73a0b` |
| `encodings/euc_jp.py` | `b453a439787b0efa031e43416a7d852a6be705c985e1200693eb96d87ea79cdc` |
| `encodings/euc_kr.py` | `633a1a5504bfad04b1ec9c96d44d4ebb3bb99066a218318e7d67d866e20887a6` |
| `encodings/gb18030.py` | `6c10b4dc49bc63724e539137ede6936304fcca1c97c28d16d89f381e10849521` |
| `encodings/gb2312.py` | `3d2d567d8d079b78f3f3b566ed52ad2f38af61bf832b7dc28858b0039a032d6b` |
| `encodings/gbk.py` | `eff9b8cbc9ad2ef2e10e96afa83d3db1f775ea044aed275b7a35574ae0d8645b` |
| `encodings/hex_codec.py` | `fc5f0a31b59efe990b86efb98936769f33dd91d912ce55b49a5a4cfc516cd047` |
| `encodings/hp_roman8.py` | `c43cce763d12e8f71a63dbc16641bd87147eaf5f9d9054ea856864b216b2735b` |
| `encodings/hz.py` | `025a9531e3046e52d3e039c0be04f9a5a74651d7683a13c7c7ebd4c7dfb5996a` |
| `encodings/idna.py` | `ea8d7f01422af0ad138e4bf58d4ae63112f3ceb68833c486d6c4d18252610fd8` |
| `encodings/iso2022_jp.py` | `461a0e7f72eccb8b29f351c4e7926cfbda58e0edd6d0770bd82e0b36c5febe77` |
| `encodings/iso2022_jp_1.py` | `63bacad13a979a5519fcaa4f1e1e07b2c7415005167fac3a689408c7d886fabd` |
| `encodings/iso2022_jp_2.py` | `5d4248181548b0fc89a9f5ee9cf52ebecb235708ba87d47896ad14130884ef9f` |
| `encodings/iso2022_jp_2004.py` | `b4d1468bcd608b46f38cb0c6ef115510dcf9aa0f71e590792f407efc6e165164` |
| `encodings/iso2022_jp_3.py` | `3aceaa5661909de14e2861d864443b8472460ce39b99cce5c6965346d47aa5ac` |
| `encodings/iso2022_jp_ext.py` | `f4c9ed8f3031995faa224bcb10153d2b6144944477d1f27d1a6cc4a879fac34c` |
| `encodings/iso2022_kr.py` | `1c86362e17944f0bcf68db02f4995bdeea605867795fff7ab4079073f96705e4` |
| `encodings/iso8859_1.py` | `b5cebd515e057d670bf54e10b8a6f162ef3daa7f21b146aee3249160caf3c32d` |
| `encodings/iso8859_10.py` | `54c886b41819ebb7f4fb34b8dbae1c45f4fc0864f019ecd772676ccfac5fae7b` |
| `encodings/iso8859_11.py` | `ed5a964470a241b4da7a6cfb718e4149d09644933af38f0497602baab6e563ef` |
| `encodings/iso8859_13.py` | `7312237e8e5d201d920b4130f057cfdf1b0be9baafaa246826e6d93204fcc206` |
| `encodings/iso8859_14.py` | `82778b995a0ee87c5f1180fcc52900359eee15bd9a6e3a0e25f0d963e0b2a343` |
| `encodings/iso8859_15.py` | `01976a81811873dc9a0c79db9fc00d1c30103487f3c6bc3a6d81b4043cd48e02` |
| `encodings/iso8859_16.py` | `b5ac8f5a5d8f84c0f903b2b7c342184758d590d8bcf810d561f942fe5b372d66` |
| `encodings/iso8859_2.py` | `2b57cab6111cae9021505e3ae1b2adbbfc344ec48165fda322f6b069fbb18adc` |
| `encodings/iso8859_3.py` | `4ffdf89004bf0c5230caa7079f7ca3142fc112f8b923ddb2c7358369d2d3c242` |
| `encodings/iso8859_4.py` | `87bd130daa0eaef3e4cb465e10cffb2bcd194ff74097e0c186b4b8eb7be41ac5` |
| `encodings/iso8859_5.py` | `9961d96cc7b9fdf011ebcaaeaeca7b50b8670fadbd7b75fde66192f8c1f68f30` |
| `encodings/iso8859_6.py` | `4840e68014346517680f593ca22f67133c39ba7e46f34b9be62c980a728448c6` |
| `encodings/iso8859_7.py` | `b352eca3b819488f64fb3338fd93f39c1e30f32bb13f2f9c577925e58f2960e4` |
| `encodings/iso8859_8.py` | `4cf9e8a8bbe04accb1c1a80853efb19ae0772d18f81e270adefc1b2386cb368e` |
| `encodings/iso8859_9.py` | `84d9b15263e81685f7513c5ab45caf80b2f73c301c68e659f7162c1b1882d359` |
| `encodings/johab.py` | `9586615917afd3d848c1c4328656603b2834af6115f2aec932fccc935e1a60fb` |
| `encodings/koi8_r.py` | `4d4e353aee8039bb71e2145a6e68fe1e6833a1b4250b70ee0ac5ec70bbb8c51d` |
| `encodings/koi8_t.py` | `9c9043814abdbe7dc39ff98f3857d5d110a84c978ad2304158d810a4e9eacef1` |
| `encodings/koi8_u.py` | `d449f9858e357fa8c2edbd4b9fe739337e9f201cac3ded20f99bfcecd4970ff7` |
| `encodings/kz1048.py` | `76beb30e98a911f72f97609a2373782573c17c88a5fb3537db338aa382979ffc` |
| `encodings/latin_1.py` | `b75503e532a27c636477396c855209ff5f3036536d2a4bede0a576c89382b60c` |
| `encodings/mac_cyrillic.py` | `83616786a1c6308b03a0dc82536908d24d0974b2248d67393d613fe558cea4bd` |
| `encodings/mac_greek.py` | `63016a323ddf98cb3aa9cfa78f3bab4768bedbfe9a5262a36a5aecb13d291f6e` |
| `encodings/mac_iceland.py` | `753cc1ac635caa7e1b4630fbcebef8db8db332c098154a5b11f652912bf64f37` |
| `encodings/mac_latin2.py` | `31670da18ce8b5394cd53fe6bf216268e7e8eae4c0247532e420e2e103727d50` |
| `encodings/mac_roman.py` | `230367d96aef8e8d7f185b4acfb84923714f39ddbcbf9cf38a06bf6f5d621c22` |
| `encodings/mac_turkish.py` | `99758a5cad2825cb3be3fa5d031e0821e4eba910a46f417fd890207b9b6be77b` |
| `encodings/palmos.py` | `79b4edcfaede8f7deb25b278d60e472217c0ff55e830ef1ccd2c408413742fa0` |
| `encodings/ptcp154.py` | `0eabcb2c287d335e86b71b0abe5718bd6ddc9aaee234f0f0f2363845d2926d8d` |
| `encodings/punycode.py` | `1e8d57e06e9b527009c35f2a1486ab56b51540e817f5bd8f239dc71e3fc0b014` |
| `encodings/quopri_codec.py` | `502a213c34c05a94ed063ee03f47680bd6efbb35036e06fb4dc809bf398cfa64` |
| `encodings/raw_unicode_escape.py` | `fa6328486b8f5a5cbd10e377e80adb8cf94acbbe19c38b4e1bf708d831a80a3a` |
| `encodings/rot_13.py` | `14767f475acdc0bf48e6272280dd15b80efaecafb93c06be21136f83dd1ee7e4` |
| `encodings/shift_jis.py` | `ad4ac50ebf58294304e412cc0f1b12980988dd6edc414e4110029c0a1abbe966` |
| `encodings/shift_jis_2004.py` | `d21c5930f21063ea78fea3b0f76dfb8fd92858d2a4a200064a52126a43dd1a99` |
| `encodings/shift_jisx0213.py` | `2c8d0b93bb36edf31c1236b1b4d1c0008553868bd2fc9137570115b96b834f2e` |
| `encodings/tis_620.py` | `647c4719e2c1a7375105e15a89b377c66f6b699977dcabbb71d923a4607b7902` |
| `encodings/undefined.py` | `0e1e3e7c1dbc4690b71494099541e73de23c9010a54b6822f3867aec35a74a62` |
| `encodings/unicode_escape.py` | `507e7ca8f18df639fd823d7cc23ce4028a3550ceefdfa40b3c76f81d1a94531d` |
| `encodings/utf_16.py` | `08437559c8d255a5ba1449c1c75beaa62b0834fe4259d161ecf8699381e4c8e1` |
| `encodings/utf_16_be.py` | `3357196f3fa52433326a6626880e34964e00c5570aee50e9a0a0a7c6d86f6e4f` |
| `encodings/utf_16_le.py` | `3aedaf3eb49769282daef1eaedfd4fa1c31fe5eebeff67fe2307c89dc2e2fd80` |
| `encodings/utf_32.py` | `6c235e377e1e277cde5f684cee6f6f8da4c73d153a21891df0cb834f17cd2b6f` |
| `encodings/utf_32_be.py` | `cbba20e1f6d0879c7c4293446c371a9f79e7c90bf3c78a77a9b8fc72b18915dd` |
| `encodings/utf_32_le.py` | `9134b91047d85b442898d59effe23e7e0cf4167ca341ae31119a731dbf880a7b` |
| `encodings/utf_7.py` | `9ff32314f4f1fa074f206bbf7fdb851504e5313128636d73b4bf75b886e4a87d` |
| `encodings/utf_8.py` | `ba0cac060269583523ca9506473a755203037c57d466a11aa89a30a5f6756f3d` |
| `encodings/utf_8_sig.py` | `1ef3da8d8aa08149e7f274dc64dbfce2155da812e5258ca8e8f832428d3b5c2d` |
| `encodings/uu_codec.py` | `45ba92000718abf85f158563c755205e100356ce1b4ab9444b4d0a3d21f061a3` |
| `encodings/zlib_codec.py` | `6ef01e8d3a5fe1cc52f7b5ae008df12f1dbce7304111bf8d4758f1bfc0115759` |
| `stringprep.py` | `52618dbafb21fac84147d4241a364b08d1f0d803806cb86e4fcb921c200c754d` |
| `encodings/cp273.py` | `6c6aec3b213ea3aebc2c526dd4d121c95d4a25a2fc928a87cd80f8448988185f` |
| `encodings/mbcs.py` | `f6ed445ed537c9f856d8defe8b56505727737d0dc9348d0a877abedab4bdd864` |



`python-3.14.8/test_pathlib/__main__.py` is a local package runner, like the dataclasses runner. It calls the unchanged upstream package's `load_tests` hook; it is not a copied CPython reference file.

`python-3.14.8/test_lumen_pathlib_{join,join_posix,join_windows,read,write,copy}.py` are local package-context runners for unchanged upstream modules. They add no assertions or skips and are not CPython reference copies.
