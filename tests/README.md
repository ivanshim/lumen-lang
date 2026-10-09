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
| `python-3.14.8/test_inspect/__init__.py` | `Lib/test/test_inspect/__init__.py`, unchanged; SHA-256 `9b2cce72fa8c7a7e688a79d55e15ea4914078c81058b0289ceb80cd5b69821c9` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_deferred_annotations.py` | `Lib/test/test_inspect/inspect_deferred_annotations.py`, unchanged; SHA-256 `e5a5fbf9ecd990cfa51812a5a4aaa23b1313af3189bfb892febd307068c8d750` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_fodder.py` | `Lib/test/test_inspect/inspect_fodder.py`, unchanged; SHA-256 `c418221061e9818de6b0624f5c5d9dbe725cc7f61431d0302fa641af94764a81` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_fodder2.py` | `Lib/test/test_inspect/inspect_fodder2.py`, unchanged; SHA-256 `66cfe2a9a93c6fb704856ee7249a38a353748954be76e6ca6b230789e7cb7cb0` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_stock_annotations.py` | `Lib/test/test_inspect/inspect_stock_annotations.py`, unchanged; SHA-256 `0c9c665429a34dc1ec6832ff33d62ed05798ce0953dd681481b6330eb84a3c62` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_stringized_annotations.py` | `Lib/test/test_inspect/inspect_stringized_annotations.py`, unchanged; SHA-256 `3100c0f9c80ee313ba939ba7870a6da629f55b3b6afc2ba2ba952706cb2bd934` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_stringized_annotations_2.py` | `Lib/test/test_inspect/inspect_stringized_annotations_2.py`, unchanged; SHA-256 `e9ea40815a00612f56b6534c812c10766ed9ba2fc497219737d1577ecc1ccbbb` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/inspect_stringized_annotations_pep695.py` | `Lib/test/test_inspect/inspect_stringized_annotations_pep695.py`, unchanged; SHA-256 `ebc8b2025fb8c53022db62cdb5bf37d6ad00cb641c5a1824aed3dc346b777258` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_inspect/test_inspect.py` | `Lib/test/test_inspect/test_inspect.py`, unchanged; SHA-256 `a0a78e87e27bdf723dbae8f9a74d9fc0c364531ddfcfc8650883a00423584787` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/__init__.py` | `Lib/asyncio/__init__.py` unchanged; SHA-256 `8148552daea881e3b33639d6e9bdab0da2021367ab694ee0604431c33c16983f` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/__main__.py` | `Lib/asyncio/__main__.py` unchanged; SHA-256 `cbe42559a5d94fd9692a24f2ba984f64623eb6caef7b1917306d0901450e4c5c` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/base_events.py` | `Lib/asyncio/base_events.py` unchanged; SHA-256 `9b20711272761ecad93248391c12532fcc7264849fcf59291f27a0a3c536ff90` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/base_futures.py` | `Lib/asyncio/base_futures.py` unchanged; SHA-256 `2f3798c4b82f5ac77647908b157c924f734f36871d98970e72849ea9a9a07856` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/base_subprocess.py` | `Lib/asyncio/base_subprocess.py` unchanged; SHA-256 `23578fd6d8647243b791583ef0c1bc997c3eef34250719e2544b04b82ff67aac` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/base_tasks.py` | `Lib/asyncio/base_tasks.py` unchanged; SHA-256 `56efac65b63db927af336fa55eb5bda93c97a2defa4734ea8d695ed20fd6712a` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/constants.py` | `Lib/asyncio/constants.py` unchanged; SHA-256 `873fc2f9e66313c3c19c337269e704f204b59f9e91d6ecbec59f68335484d338` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/coroutines.py` | `Lib/asyncio/coroutines.py` unchanged; SHA-256 `c5b8563c88e343f3076630aa6545fc77c548ceefb76936a360d7895c13691753` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/events.py` | `Lib/asyncio/events.py` unchanged; SHA-256 `62e2c619c38407309c0d2c98638642174ee7be5b874676c0225392fed4a00b0f` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/exceptions.py` | `Lib/asyncio/exceptions.py` unchanged; SHA-256 `a5971f88be14cd1417d59adf539ae48c5d818f95362a4e0eb00017e3690ab37b` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/format_helpers.py` | `Lib/asyncio/format_helpers.py` unchanged; SHA-256 `c02f2f8c1e2508ddb2f8d8c1bef452be1f6b9a1d2a998b89a56d153e42a5f1ef` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/futures.py` | `Lib/asyncio/futures.py` unchanged; SHA-256 `2f482524292a7c5d1b818aeb92434d183bd089e014cf50b785432284d2c8bc12` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/graph.py` | `Lib/asyncio/graph.py` unchanged; SHA-256 `8583a402fcd5d0bc296f753b6f05af1225459a00b1d18062a460862e87863897` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/locks.py` | `Lib/asyncio/locks.py` unchanged; SHA-256 `8ded089e680b9b67fca60893071356fa29343539f5113e66cf9da5f6ab734285` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/log.py` | `Lib/asyncio/log.py` unchanged; SHA-256 `80e4cc3ded4b138baba486519e7444801a23d6ac35f229d336a407a96af7e8d2` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/mixins.py` | `Lib/asyncio/mixins.py` unchanged; SHA-256 `8f4a3e16eca845ebfba422550cbcee7340ec8166d2bff6b750a8ed0de6b9ae3c` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/proactor_events.py` | `Lib/asyncio/proactor_events.py` unchanged; SHA-256 `e3bedbdeb8f7e18a51d7326d460741e5bcb0ad9ec0523b9fdee3a330a54076f8` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/protocols.py` | `Lib/asyncio/protocols.py` unchanged; SHA-256 `1d1b49988c338b4ef06e30f9e92d9db2e00080c341f0a3f573bb8312deb8aff6` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/queues.py` | `Lib/asyncio/queues.py` unchanged; SHA-256 `9ce87bf63f113509d479ea25fecd25a705766d750c304a180d1bff02a0fe6f6a` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/runners.py` | `Lib/asyncio/runners.py` unchanged; SHA-256 `d3b0765c4bd022dac4d101a034ba8c16b6e9c598e44d7b9d82b3239f3106fe65` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/selector_events.py` | `Lib/asyncio/selector_events.py` unchanged; SHA-256 `de05a1bec1f96eb85643d67958b31a01f0dff3210af0527af5ff3629bcbaa17e` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/sslproto.py` | `Lib/asyncio/sslproto.py` unchanged; SHA-256 `b33d2f13fc373b7220402c4358dc96c770cea14489f5570b49774b37f9ac052f` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/staggered.py` | `Lib/asyncio/staggered.py` unchanged; SHA-256 `8be17b2fa59a3a207b2e004f278257fc220c592afc8b6aa1cbe47030e1b9b4a9` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/streams.py` | `Lib/asyncio/streams.py` unchanged; SHA-256 `8e36342dc19e253705f76fb164dc3abe5f26d82afbedb792477c0d530ceb62a6` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/subprocess.py` | `Lib/asyncio/subprocess.py` unchanged; SHA-256 `7b70605716334f63cc482123b2aaa3b7c5bb7138eeab63a037bd8068d43307c1` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/taskgroups.py` | `Lib/asyncio/taskgroups.py` unchanged; SHA-256 `df0ea4f0911ea9c1006b4dec45557e0862a09d7a411c63259943731df8c41dda` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/tasks.py` | `Lib/asyncio/tasks.py` unchanged; SHA-256 `de76790a02ed881e595c31272bca50854aadadd9d91ca6f52706606505cb7ab0` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/threads.py` | `Lib/asyncio/threads.py` unchanged; SHA-256 `95877db9bf2c36ce9e630780ae145627f1401b95a9e8ee101b018df2898e8072` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/timeouts.py` | `Lib/asyncio/timeouts.py` unchanged; SHA-256 `dc1544c3c27ace84e2abc10f8fd13e340709a8579ff008ebc64c0d0fae6c0837` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/tools.py` | `Lib/asyncio/tools.py` unchanged; SHA-256 `6e74b35d25157f99335d70e2352a11d9be4aeee80f3300294dc80f75862dd00b` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/transports.py` | `Lib/asyncio/transports.py` unchanged; SHA-256 `bfd414b25979e07b74081140bf91332957a9bf8b5aa5a0c7ff0d779d93eada21` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/trsock.py` | `Lib/asyncio/trsock.py` unchanged; SHA-256 `c0eac37debcc51b702b808f6b7ed3e417343f5ff5f57125dad600a27eb082328` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/unix_events.py` | `Lib/asyncio/unix_events.py` unchanged; SHA-256 `760967defab5b28d611c46d7dbd51fd734679837a87a0673837ca5bdff8b1a41` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/windows_events.py` | `Lib/asyncio/windows_events.py` unchanged; SHA-256 `c88b8c0f6bf0619649587ffba1e4623d3fa621036470ce987a0ac7269a211cb2` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/asyncio/windows_utils.py` | `Lib/asyncio/windows_utils.py` unchanged; SHA-256 `b9c40e077d5c00dd22a1058ffd3fad8e21c8753b0710f02d97ade7bfdad03493` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/concurrent/futures/thread.py` | `Lib/concurrent/futures/thread.py` unchanged; SHA-256 `7f95079dda8064cd372c9c0ea60c7e172b34c8f2aa0c3d9213e85b8584aa033c` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/concurrent/futures/process.py` | `Lib/concurrent/futures/process.py` unchanged; SHA-256 `1ca05b878a4bdc37c063d1fff8fd1d6566dcba6316049779d9b8bd15ccca14b3` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/concurrent/futures/interpreter.py` | `Lib/concurrent/futures/interpreter.py` unchanged; SHA-256 `424fac5bf71743b4e02cfa8150919cac1a9e53e2e5173729f7ba75dd2da78e1e` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/_pickle.py` | Python runtime binding to the unchanged `pickle.py` engines; SHA-256 `068df99aba92205b3e1c0e9df5c75b78cfc7bcf174125c91d9e2babb1999f217` | Runtime adapter | Local interface | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/runtime_adapters/script_helper.py` | Unchanged `kill_python` definition from `Lib/test/support/script_helper.py`, over the existing native argument adapter; SHA-256 `0a2262de2cbc68b0d0efd71963be2175c0100aa846b9f5762457daadecc47e88` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/runtime_adapters/test_package.py` | Package-path binding for relocated reference suites; SHA-256 `da74ed870675c536d297f274c0f897df246ac432c25ec288bc1cb52231a02705` | Runtime adapter | Local suite layout | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_mimetypes.py` | `Lib/test/test_mimetypes.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/mime.types`, `mime.types2` | `Lib/test/mime.types`, `Lib/test/mime.types2`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_genericclass.py` | `Lib/test/test_genericclass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_structseq.py` | `Lib/test/test_structseq.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_flufl.py` | `Lib/test/test_flufl.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_tabnanny.py` | `Lib/test/test_tabnanny.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_strftime.py` | [python/cpython](https://github.com/python/cpython) `Lib/test/test_strftime.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_except_star.py` | `Lib/test/test_except_star.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, `test_csv.py`, `test_configparser.py`, `test_linecache.py`, `test_tokenize.py`, `test_abc.py`, `test_contextlib.py`, `test_ordered_dict.py`, `test_defaultdict.py`, `test_glob.py`, `test_timeit.py`, `test_datetime.py`, `test_strptime.py`, `test_getopt.py`, `test_optparse.py`, and support data (`mathdata/`, `tokenizedata/`, `configdata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_raise.py`, `test_property.py` | `Lib/test/test_raise.py`, `Lib/test/test_property.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_hash.py` | `Lib/test/test_hash.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_getpass.py` | `Lib/test/test_getpass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/getpass.py` | `Lib/getpass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_iterlen.py` | `Lib/test/test_iterlen.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_xml_dom_minicompat.py` | `Lib/test/test_xml_dom_minicompat.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_metaclass.py` | `Lib/test/test_metaclass.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_sundry.py` | `Lib/test/test_sundry.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/` | [python/cpython](https://github.com/python/cpython) `Lib/timeit.py` and the library files it and its test import that the suite lacked: `Lib/getopt.py`, `Lib/gettext.py`, `Lib/linecache.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/xml/` | [python/cpython](https://github.com/python/cpython) `Lib/xml/__init__.py`, `Lib/xml/dom/__init__.py`, `Lib/xml/dom/domreg.py`, `Lib/xml/dom/minicompat.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_copyreg.py`, `python-3.14.8/test/{pickletester,picklecommon}.py` | `Lib/test/test_copyreg.py` and the pickle support classes it imports, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test/` | `Lib/test/__init__.py` and `Lib/test/support/{__init__,import_helper,i18n_helper,threading_helper,os_helper,script_helper}.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_tomllib/` | [python/cpython](https://github.com/python/cpython) `Lib/test/test_tomllib`: `__init__.py`, `__main__.py`, `burntsushi.py`, `test_data.py`, `test_error.py`, `test_misc.py` and the `data/` tree of `.toml`/`.json` fixtures | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/tomllib/` | [python/cpython](https://github.com/python/cpython) `Lib/tomllib`: `__init__.py`, `_parser.py`, `_re.py`, `_types.py`, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test_email/`, `langs/lib_python/modules/test/test_email/` | `Lib/test/test_email/`: every Python source and `data/` fixture, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/socket.py`: `getfqdn` function | `Lib/socket.py`: unchanged function in the native adapter | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/test/support/__init__.py`: `patch` | `Lib/test/support/__init__.py`: helper function copied unchanged into the existing runtime adapter | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/email/` | `Lib/email/`: every Python source, copied byte for byte | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `langs/lib_python/modules/codecs.py`, `stringprep.py`, `encodings/` | `Lib/codecs.py`, `Lib/stringprep.py`, and all 115 `Lib/encodings/` wrapper bodies, copied byte for byte; reused from the signed codec port | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

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
The `_pickle` interface now binds the released pickle module's Python engines
through lazy attributes to avoid the accelerator import cycle. It implements
real serialization, not C acceleration: C-only descriptor and type metadata
remain unavailable and those assertions fail honestly.
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
| `dis.py` | runtime adapter / implementation | `3b72c1a94e25a5a4772e515c1dd9bf823b3ac978fe77a2ec9a6f49173b0aed02` | `17d4d9a9195c9ee5aa27a96f3e0871f9363fd501e3a64ef20cb1cc876d7ad553` |
| `doctest.py` | runtime adapter / implementation | `ad615d7193536046381d5d66eadab014bcd7edf7b1815add0cf7c50750fd7e03` | `3a5c30736974de19debbc9b3600461eef1943bd8b3b8afe3a634c05124b5dd5c` |
| `enum.py` | runtime adapter / implementation | `80ebe6d718e9601a77cc56c8a7a526b9e2938eebb0e6d308aded60ca4cabad11` | `fd23a7598fa1104ef892abcd4154d3627283e6361eb7a91cd11dc4a7b6fb3a93` |
| `errno.py` | runtime module; no Lib counterpart | `e9dac45709d8d26e173a71b7dc5b8507848e3729cfe7e591d8083992efe38d10` | `—` |
| `fnmatch.py` | unchanged release copy | `ce582bc266922e4c682e2a85d72095124430a6bf58716c6f2537103480eae742` | `ce582bc266922e4c682e2a85d72095124430a6bf58716c6f2537103480eae742` |
| `fractions.py` | runtime adapter / implementation | `503db9d4f73590da7bbbed2b1ff167348f24ebf256c5fbc373c5181e9f34f5ab` | `7a95f1c506c9ac4b2277df5f2bdd9d61cc67b520c45021a5a961939770221ef6` |
| `functools.py` | unchanged release copy | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` |
| `gc.py` | runtime module; no Lib counterpart | `a879e867e999268f591a51151fb5efe9d51a7046d9d155a6a5f58bfd76ef336f` | `—` |
| `glob.py` | unchanged release copy | `21d290be171e3d0643a444e4b58a2c4ee9bfe2c1474bb36ead043794bb769365` | `21d290be171e3d0643a444e4b58a2c4ee9bfe2c1474bb36ead043794bb769365` |
| `heapq.py` | unchanged release copy | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` |
| `inspect.py` | unchanged release source | `6daf297029d8971703b344de5cd485d71d4d935c72b85591493d8abc8512bdf7` | `6daf297029d8971703b344de5cd485d71d4d935c72b85591493d8abc8512bdf7` |
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
| `runtime_adapters/inspect.py` | runtime adapter / implementation | `7bd576508bf126c7040f4486126a5225b4a86cd2758f54167c883d82915560f2` | `6daf297029d8971703b344de5cd485d71d4d935c72b85591493d8abc8512bdf7` |
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
| `unittest/mock.py` | runtime adapter / implementation | `1cd52e4f3568e9bf3e081f24129fa5fba53c517fae4ab6de4ce6b0d98444a929` | `2717e1bf1544d3173d10d467b3d95ed99da2d4428767f9f516de2080033408b1` |
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


## Inspect runtime source

The runtime now loads the complete, unchanged v3.14.8 `inspect.py` rather
than the former partial native replacement. `runtime_adapters/inspect.py`
contains the remaining adapter needs: partialmethod handles retain their
native wrapper shape, and native disassembly currently lacks global/attribute
reference kinds. Frame positions use retained native instruction offsets
rather than CPython two-byte offsets. Closure cells can be inspected, but functions with named
references raise `NotImplementedError` instead of silently losing globals.
Native annotation evaluators can refuse unresolved FORWARDREF signatures;
this adapter reports `ValueError` for that unavailable signature instead of
letting evaluation errors escape from callers that handle uninspectable
callables. Only a failed fallback from an unsupported format is translated;
direct callback errors and VALUE annotation errors retain their exception.
Type flags include the abstract bit from the class's own abstract-method set.
The abstract marker is read from the class itself while subclass hooks run,
so an ancestor's completed marker does not suppress unfinished-class analysis.
The kernels expose asynchronous-generator suspension state and retain the
v3.14 docstring and method code flags, with class scopes excluded from nested
function flags. Frames expose their retained native `f_lasti`; `dis.Positions`
is the release named-tuple interface. `dis.py` exposes the flag constants.

All nine upstream `Lib/test/test_inspect/` files are present unchanged.
There is no `__main__.py` in that upstream directory; the runnable test file
is `test_inspect/test_inspect.py`. The asyncio package is now the complete
unchanged release package, using its Python Future/Task fallback when `_asyncio`
is absent. Native socket operations and isolated-test event-loop suspension
remain limited; importing the package does not claim support for those operations.
The `_pickle` interface binds the real Python serialization engines.
Circular imports ignore unbound module `__getattr__` slots, and native frame
types expose the standard `builtins` module name used by annotation formatting.
Other remaining limits include native builtin text signatures and syntax-tree
parsing for their signatures, string-format annotations, and source/frame
operations that require information the runtime does not retain.

The `test` package adapter extends its real package path with reference-suite
roots containing `test/__init__.py` and `test/support/`, discovered from the
program path and `sys.path`. This supports the repository's relocated test
packages without changing their imports. The separate script-helper overlay
adds the unchanged released `kill_python` function; the owned subprocess
argument-routing implementation stays intact.

The batch-21f signature additions are provided by the released inspect
implementation: Parameter fields and replacement, immutable parameter mappings,
Signature replacement/equality/hash, annotation formatting and metaclass call
signatures. The redundant native replacement is removed; manifest registration
continues to bind the release source and its separate runtime overlay.

Native union values inherit the v3.14 `typing` module marker from their
canonical union class. This lets the unchanged inspect formatter remove
outer typing prefixes while preserving typing inside user module names.

Microcode7 frame-local snapshots include the routine's captured cells, using
the same lexical addresses as ordinary local lookup. This exposes live
closure bindings before a coroutine starts and while a generator is suspended;
unbound cells stay absent. Stack8 already provides these bindings.

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
