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
| `python-3.14.8/` | [python/cpython](https://github.com/python/cpython) `Lib/test`: core-language files, `test_functools.py`, `test_operator.py`, `test_heapq.py`, `test_bisect.py`, `test_copy.py`, `test_keyword.py`, `test_itertools.py`, and support data (`mathdata/`) | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |
| `python-3.14.8/test/` | `Lib/test/__init__.py` and `Lib/test/support/{__init__,import_helper,threading_helper,os_helper,script_helper}.py` | 3.14.8 / `v3.14.8` | `8e6e75d9102e` / 2026-09-30 | [PSF](python-3.14.8/LICENSE) |

The `python-3.14.8/test/` package and its support, import, threading, OS, and script helpers
are also preserved byte for byte at that commit. The embedded runtime support
modules in `langs/lib_python/modules/test/` provide the interpreter adapters.
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

The report runs every registered suite with its exact `--python` pin. To select
one suite, pass `--python 3.14`; `--binary target/debug/lumen-lang --no-build`
uses an existing debug build. Count tools accept the same `--python` option,
then `LUMEN_PYTHON`, then the newest registered release. Never edit a reference
file. A bugfix refresh uses `git mv` on its full-release directory and replaces
the table pin; it does not keep an older micro release. Add a provenance table
here for each newly registered series.
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
| `cmath.py` | runtime adapter / implementation | `c5a49f40a56b551c4ae6309a800f66e5d3f46a3036297754e93abb093d20494e` | `9687600b7e34cb6d2fefff71c32fa9f7d5646ffa64a38556f4f6e78f737161af` |
| `codecs.py` | runtime adapter / implementation | `4aa54c3ba26e60d235e73bbdb088af5f4a6f665eb5ad4b58263f9a7524128733` | `718f39b3ea68fe934214d789c5bce3005fa828040b03a72d2f715fc5e2647b7e` |
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
| `doctest.py` | runtime adapter / implementation | `80d3ebd7686e0c224e35878ba637d0e2165e8e8e0bc65eaf7236ac444d5ed950` | `3a5c30736974de19debbc9b3600461eef1943bd8b3b8afe3a634c05124b5dd5c` |
| `enum.py` | runtime adapter / implementation | `80ebe6d718e9601a77cc56c8a7a526b9e2938eebb0e6d308aded60ca4cabad11` | `fd23a7598fa1104ef892abcd4154d3627283e6361eb7a91cd11dc4a7b6fb3a93` |
| `errno.py` | runtime module; no Lib counterpart | `e9dac45709d8d26e173a71b7dc5b8507848e3729cfe7e591d8083992efe38d10` | `—` |
| `fnmatch.py` | unchanged release copy | `ce582bc266922e4c682e2a85d72095124430a6bf58716c6f2537103480eae742` | `ce582bc266922e4c682e2a85d72095124430a6bf58716c6f2537103480eae742` |
| `fractions.py` | runtime adapter / implementation | `503db9d4f73590da7bbbed2b1ff167348f24ebf256c5fbc373c5181e9f34f5ab` | `7a95f1c506c9ac4b2277df5f2bdd9d61cc67b520c45021a5a961939770221ef6` |
| `functools.py` | unchanged release copy | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` | `9db56d38172c4c9e689b21cc58c8538008b09d86b682faf4dd193b529cdd79d5` |
| `gc.py` | runtime module; no Lib counterpart | `a879e867e999268f591a51151fb5efe9d51a7046d9d155a6a5f58bfd76ef336f` | `—` |
| `heapq.py` | unchanged release copy | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` | `f2d644de141a488db66fc13608a794ef5f2d33162299f6751ea92c3cb0b4c9ea` |
| `inspect.py` | runtime adapter / implementation | `ef2deb2563813252ddd1fcc82e9347d22c4be4a0c90930a6690d80446d1f7f31` | `6daf297029d8971703b344de5cd485d71d4d935c72b85591493d8abc8512bdf7` |
| `io.py` | runtime adapter / implementation | `cf85cf25e6255e6793812652b1b2e8fce04946c90f88480c76b9ad7cb29ebba1` | `1b75584d4efcc612dc0db2ae98619408276585a106f0544879edb5e25b55430a` |
| `itertools.py` | runtime adapter / implementation | `43134d3fce836a09f53e800b9b5499e649232527540ded6aa5289682f5957704` | `11e8e8186a779a26ffd33bb5042b286bd08750a946c1398756225450d6bee6cc` |
| `json.py` | runtime adapter / implementation | `f5f9bf3a3ad059f0d80ac41dd60ba4fde5da461b8fc79d5f217243aa1fd324d4` | `2dd10f1bf4c9ea5478e589216805e7f279d0e4bce134a19efa297404fb87407d` |
| `keyword.py` | unchanged release copy | `18c2be738c04ad20ad375f6a71db34b3823c7f40b0340f5294d0e89f3c9b093b` | `18c2be738c04ad20ad375f6a71db34b3823c7f40b0340f5294d0e89f3c9b093b` |
| `locale.py` | runtime adapter / implementation | `cfd2a69a3010804d881928cce4fb4a73213fa21705d056ae9e490a2cae70efe2` | `f2e390ebde2bb52eabcf6d444a6a2e758749f3aa82c61427cbb053332fac56b2` |
| `marshal.py` | runtime module; no Lib counterpart | `627650ebff7594e5c03b1499cc8d7600787a1906aeaabb557f5f568d5347c76a` | `—` |
| `math.py` | runtime module; no Lib counterpart | `2d4a139f2ed29486305b3a3121e590ea552fe71e6ad7bc0c6a67fe4a432c3c96` | `—` |
| `numbers.py` | runtime adapter / implementation | `c3a6aa26be4af50771bf76b7bb556c40d95a3432ab8c11b9072c356c9effd4ce` | `e5e73beba4a7674bd5e9a881a202d403a0c3e2b59af4181699155ea4827a7561` |
| `operator.py` | unchanged release copy | `a9f9910965c31f841caad0447ee47299ac539a371422aefe200c86c60f3b4697` | `a9f9910965c31f841caad0447ee47299ac539a371422aefe200c86c60f3b4697` |
| `os.py` | runtime adapter / implementation | `a2d7473b7b5a05e296b58980790ccfc38feeab6a93039187725e3c18517ab827` | `976bdb3e24925f2fb3ce43a4d788de6dcbb88a7cc8604c60ae690fbdd6507546` |
| `os/path.py` | runtime adapter / implementation | `bf2ce27e89674f859c937b52bac276b34002f1b34c2c1f7a11c39ede20235275` | `8a5bc2b6674e76efe0f507b873a41d746beba466a191153ed18a7580ef6e08e1` |
| `pdb.py` | runtime adapter / implementation | `1a058afea49ceca837b238c1bc0f6aa5cc04119ff283f7935b198946b7023d50` | `50d5e0f9dab1f4e2d114c6e6d62577d8c76018c8732443a48bf783671179317f` |
| `pickle.py` | runtime adapter / implementation | `5347a713d3192ff3044178d5e04e9e5dc0701d17cfc477d6fefac03293c8c16f` | `c9998751cf3b94536b4c2cffd6bb719fa1ee6027bd1bd37235d597c9ecf9f98c` |
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
| `shlex.py` | unchanged release copy | `aeda1c54188363d907654a19d338e5961416ebec2e2f506afab19be61ada8120` | `aeda1c54188363d907654a19d338e5961416ebec2e2f506afab19be61ada8120` |
| `shutil.py` | runtime adapter / implementation | `adf9e03edd25763490ee7d77f27fc8f8bdebf4bd4d0b2c7226699eb07bda968a` | `28a5df6415bf1a7ab36cbd544d6d35ae0f056a198e3872a786dc4e156d5fcd7d` |
| `signal.py` | runtime adapter / implementation | `827449d3ecf3086232ec54bbe07e16ea5987f08ff70be9ff8f27e0ace8ec4214` | `0363c964c90ac0b3e515de5749205e6e6454051a1211058375d84d91eab6071a` |
| `statistics.py` | runtime adapter / implementation | `0d3d742eba317f6d8efda263bf040df59b6d051fc511d5ccf7c934ba9ae289b2` | `b7558385a2005432c0ae74a222866316bf4f9c3ed448555659e7014b1077173a` |
| `string.py` | runtime adapter / implementation | `87cf93090f882c1dc39ad9b2a9cd0ef9770415936b0aec421617a2cc6c5e9540` | `8c9751e6ae9776b4e94c1c9850b53c0d24e8407196901aeaebffbb1a11a64e05` |
| `struct.py` | runtime adapter / implementation | `400595da579c406bbe6f5a3dec52d7d728910d4927e878acad68678c0cc8f0d0` | `50c14e55f94957804c1e45e975fedbe8e86f7e1dbb745b150370ff681155370a` |
| `subprocess.py` | runtime adapter / implementation | `b3947dd75e8d4b84a7e4e84e522c1a5a13f7880571b4f820d3da5f3a2e8e8b2e` | `7fa4af83422024a19e64a80937f6c8094244c3397372a495b348dfbe44a5dd72` |
| `sys.py` | runtime module; no Lib counterpart | `90019bb1d77e139ef774cc82b3d01e5206435529c869732c137bd8777b5eb443` | `—` |
| `tempfile.py` | runtime adapter / implementation | `bc171d211898ecf33be9df1c08a48ce09e3b9e0852e13447b4f508e8d9e1fb63` | `3714372b63f6bc05a5273425d446f6ad3a2112a659c71c321bb4575fd082d634` |
| `test/__init__.py` | restored release copy | `836cdb388117cf81e78d9fa2a141cca1b14b0179733322e710067749a1b16fe9` | `836cdb388117cf81e78d9fa2a141cca1b14b0179733322e710067749a1b16fe9` |
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
| `test/support/script_helper.py` | runtime adapter / implementation | `4ce0a8771b3aaf48ffc2fd01a00b423c24b281f6f849a38df65603344b6a7e55` | `d94c1502c2b7a3f1e57deb1c8913e1890049c73979261f205fc2f7e96fc03677` |
| `test/support/testcase.py` | restored release copy | `69683bb4a66f7abfb91c5726b0e6c2434a2e1bd2d7ddda5b88b602b1577be12c` | `69683bb4a66f7abfb91c5726b0e6c2434a2e1bd2d7ddda5b88b602b1577be12c` |
| `test/support/threading_helper.py` | runtime adapter / implementation | `32b013b39f834663bd1f253ca11cc18e99a5cdb558178e1a35b8ada372442f76` | `93976321a0592dda85d769089d443f7ce6f0742911852d3d875de5f03a8d7471` |
| `test/support/warnings_helper.py` | runtime adapter / implementation | `065324aee43a5c679cf698c0155583b064960153f7f37ae83589b01fdfbf2807` | `fc02de4d91bae3988079e3fb3fec3da96ae467fd548295745c2846af179f3870` |
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
| `unittest.py` | runtime adapter / implementation | `59844316763609ebd8d74409e324c207e9305f76a973b5e32efbebffffad554c` | `2698f2daf7a02609a6825b89cd459ebaa283dec2403c7b846315a0701bc74a0d` |
| `unittest/mock.py` | runtime adapter / implementation | `1cd52e4f3568e9bf3e081f24129fa5fba53c517fae4ab6de4ce6b0d98444a929` | `856148cdc93943b4ff948276e94561db3d6c44ddecacf53c3d25eb7dc46a108d` |
| `warnings.py` | runtime adapter / implementation | `ee0bf5d9c33be43a8deec328c8970c4570ef75c9496979a7abeed64aafbfad6b` | `142225786de63c593f1c9abdacf5b4fc0b05dd847f6bed0ebb4b4aa2d4d93b02` |
| `weakref.py` | runtime adapter / implementation | `e080fe0eeefbc2018cfe0a90f71d159599c4b225e4c319289f3896dec7383728` | `5e5f727a19a858cb4c56dbaf3e0a138ded02fb954a9a58a840a0764216ae9522` |

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
