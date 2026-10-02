# CPython source inventory

Inventory: 120 source/provenance entries: 107 primary files and 13 scratch copies.
105 release bodies are verified in sha256.txt; two absent fixtures were removed;
13 documented partial adapters remain.

Old pin: 3b564385e4c9. Target: v3.14.8 / 8e6e75d9102e (2026-09-30).

| Local file | Upstream path | Kind | Old bytes match | Release exists |
|---|---|---|---|---|
| `tests/python/LICENSE` | `LICENSE` | test | True | True |
| `tests/python/mathdata/cmath_testcases.txt` | `Lib/test/mathdata/cmath_testcases.txt` | test | True | True |
| `tests/python/mathdata/floating_points.txt` | `Lib/test/mathdata/floating_points.txt` | test | True | True |
| `tests/python/mathdata/formatfloat_testcases.txt` | `Lib/test/mathdata/formatfloat_testcases.txt` | test | True | True |
| `tests/python/mathdata/ieee754.txt` | `Lib/test/mathdata/ieee754.txt` | test | True | True |
| `tests/python/mathdata/math_testcases.txt` | `Lib/test/mathdata/math_testcases.txt` | test | True | True |
| `tests/python/test/__init__.py` | `Lib/test/__init__.py` | test | True | True |
| `tests/python/test/support/__init__.py` | `Lib/test/support/__init__.py` | test | True | True |
| `tests/python/test/support/import_helper.py` | `Lib/test/support/import_helper.py` | test | True | True |
| `tests/python/test/support/os_helper.py` | `Lib/test/support/os_helper.py` | test | True | True |
| `tests/python/test/support/script_helper.py` | `Lib/test/support/script_helper.py` | test | True | True |
| `tests/python/test/support/threading_helper.py` | `Lib/test/support/threading_helper.py` | test | True | True |
| `tests/python/test_augassign.py` | `Lib/test/test_augassign.py` | test | True | True |
| `tests/python/test_bigmem.py` | `Lib/test/test_bigmem.py` | test | True | True |
| `tests/python/test_binop.py` | `Lib/test/test_binop.py` | test | True | True |
| `tests/python/test_bisect.py` | `Lib/test/test_bisect.py` | test | True | True |
| `tests/python/test_bool.py` | `Lib/test/test_bool.py` | test | True | True |
| `tests/python/test_builtin.py` | `Lib/test/test_builtin.py` | test | True | True |
| `tests/python/test_class.py` | `Lib/test/test_class.py` | test | True | True |
| `tests/python/test_cmath.py` | `Lib/test/test_cmath.py` | test | True | True |
| `tests/python/test_compare.py` | `Lib/test/test_compare.py` | test | True | True |
| `tests/python/test_complex.py` | `Lib/test/test_complex.py` | test | True | True |
| `tests/python/test_contains.py` | `Lib/test/test_contains.py` | test | True | True |
| `tests/python/test_copy.py` | `Lib/test/test_copy.py` | test | True | True |
| `tests/python/test_decorators.py` | `Lib/test/test_decorators.py` | test | True | True |
| `tests/python/test_dict.py` | `Lib/test/test_dict.py` | test | True | True |
| `tests/python/test_dictcomps.py` | `Lib/test/test_dictcomps.py` | test | True | True |
| `tests/python/test_enumerate.py` | `Lib/test/test_enumerate.py` | test | True | True |
| `tests/python/test_eof.py` | `Lib/test/test_eof.py` | test | True | True |
| `tests/python/test_exceptions.py` | `Lib/test/test_exceptions.py` | test | True | True |
| `tests/python/test_float.py` | `Lib/test/test_float.py` | test | True | True |
| `tests/python/test_fnmatch.py` | `Lib/test/test_fnmatch.py` | test | True | True |
| `tests/python/test_format.py` | `Lib/test/test_format.py` | test | True | True |
| `tests/python/test_fractions.py` | `Lib/test/test_fractions.py` | test | True | True |
| `tests/python/test_fstring.py` | `Lib/test/test_fstring.py` | test | True | True |
| `tests/python/test_funcattrs.py` | `Lib/test/test_funcattrs.py` | test | True | True |
| `tests/python/test_functools.py` | `Lib/test/test_functools.py` | test | True | True |
| `tests/python/test_generators.py` | `Lib/test/test_generators.py` | test | True | True |
| `tests/python/test_genexps.py` | `Lib/test/test_genexps.py` | test | True | True |
| `tests/python/test_global.py` | `Lib/test/test_global.py` | test | True | True |
| `tests/python/test_grammar.py` | `Lib/test/test_grammar.py` | test | True | True |
| `tests/python/test_heapq.py` | `Lib/test/test_heapq.py` | test | True | True |
| `tests/python/test_import/data/syntax_warnings.py` | `Lib/test/test_import/data/syntax_warnings.py` | test | True | False |
| `tests/python/test_index.py` | `Lib/test/test_index.py` | test | True | True |
| `tests/python/test_int.py` | `Lib/test/test_int.py` | test | True | True |
| `tests/python/test_int_literal.py` | `Lib/test/test_int_literal.py` | test | True | True |
| `tests/python/test_iter.py` | `Lib/test/test_iter.py` | test | True | True |
| `tests/python/test_itertools.py` | `Lib/test/test_itertools.py` | test | True | True |
| `tests/python/test_keyword.py` | `Lib/test/test_keyword.py` | test | True | True |
| `tests/python/test_keywordonlyarg.py` | `Lib/test/test_keywordonlyarg.py` | test | True | True |
| `tests/python/test_list.py` | `Lib/test/test_list.py` | test | True | True |
| `tests/python/test_listcomps.py` | `Lib/test/test_listcomps.py` | test | True | True |
| `tests/python/test_long.py` | `Lib/test/test_long.py` | test | True | True |
| `tests/python/test_math.py` | `Lib/test/test_math.py` | test | True | True |
| `tests/python/test_opcodes.py` | `Lib/test/test_opcodes.py` | test | True | True |
| `tests/python/test_operator.py` | `Lib/test/test_operator.py` | test | True | True |
| `tests/python/test_positional_only_arg.py` | `Lib/test/test_positional_only_arg.py` | test | True | True |
| `tests/python/test_pow.py` | `Lib/test/test_pow.py` | test | True | True |
| `tests/python/test_print.py` | `Lib/test/test_print.py` | test | True | True |
| `tests/python/test_range.py` | `Lib/test/test_range.py` | test | True | True |
| `tests/python/test_scope.py` | `Lib/test/test_scope.py` | test | True | True |
| `tests/python/test_set.py` | `Lib/test/test_set.py` | test | True | True |
| `tests/python/test_setcomps.py` | `Lib/test/test_setcomps.py` | test | True | True |
| `tests/python/test_shlex.py` | `Lib/test/test_shlex.py` | test | True | True |
| `tests/python/test_slice.py` | `Lib/test/test_slice.py` | test | True | True |
| `tests/python/test_str.py` | `Lib/test/test_str.py` | test | True | True |
| `tests/python/test_string_literals.py` | `Lib/test/test_string_literals.py` | test | True | True |
| `tests/python/test_syntax.py` | `Lib/test/test_syntax.py` | test | True | True |
| `tests/python/test_textwrap.py` | `Lib/test/test_textwrap.py` | test | True | True |
| `tests/python/test_tuple.py` | `Lib/test/test_tuple.py` | test | True | True |
| `tests/python/test_unary.py` | `Lib/test/test_unary.py` | test | True | True |
| `tests/python/test_unpack.py` | `Lib/test/test_unpack.py` | test | True | True |
| `tests/python/test_with.py` | `Lib/test/test_with.py` | test | True | True |
| `langs/lib_python/modules/_bisect.py` | `Lib/bisect.py` | adapter | False | True |
| `langs/lib_python/modules/_heapq.py` | `Lib/heapq.py` | adapter | False | True |
| `langs/lib_python/modules/_namedtuple.py` | `Lib/collections/__init__.py` | adapter | False | True |
| `langs/lib_python/modules/annotationlib.py` | `Lib/annotationlib.py` | adapter | False | True |
| `langs/lib_python/modules/bisect.py` | `Lib/bisect.py` | copy | True | True |
| `langs/lib_python/modules/copy.py` | `Lib/copy.py` | adapter | False | True |
| `langs/lib_python/modules/copyreg.py` | `Lib/copyreg.py` | adapter | False | True |
| `langs/lib_python/modules/fnmatch.py` | `Lib/fnmatch.py` | copy | True | True |
| `langs/lib_python/modules/functools.py` | `Lib/functools.py` | adapter | False | True |
| `langs/lib_python/modules/heapq.py` | `Lib/heapq.py` | adapter | False | True |
| `langs/lib_python/modules/itertools.py` | `Modules/itertoolsmodule.c` | adapter | False | True |
| `langs/lib_python/modules/keyword.py` | `Lib/keyword.py` | copy | True | True |
| `langs/lib_python/modules/operator.py` | `Lib/operator.py` | adapter | False | True |
| `langs/lib_python/modules/posixpath.py` | `Lib/posixpath.py` | adapter | False | True |
| `langs/lib_python/modules/reprlib.py` | `Lib/reprlib.py` | copy | True | True |
| `langs/lib_python/modules/shlex.py` | `Lib/shlex.py` | copy | True | True |
| `langs/lib_python/modules/test/list_tests.py` | `Lib/test/list_tests.py` | adapter | False | True |
| `langs/lib_python/modules/test/mapping_tests.py` | `Lib/test/mapping_tests.py` | adapter | False | True |
| `langs/lib_python/modules/test/mathdata/cmath_testcases.txt` | `Lib/test/mathdata/cmath_testcases.txt` | copy | True | True |
| `langs/lib_python/modules/test/mathdata/math_testcases.txt` | `Lib/test/mathdata/math_testcases.txt` | copy | True | True |
| `langs/lib_python/modules/test/seq_tests.py` | `Lib/test/seq_tests.py` | adapter | False | True |
| `langs/lib_python/modules/test/string_tests.py` | `Lib/test/string_tests.py` | copy | True | True |
| `langs/lib_python/modules/test/support/__init__.py` | `Lib/test/support/__init__.py` | adapter | False | True |
| `langs/lib_python/modules/test/support/numbers.py` | `Lib/test/support/numbers.py` | copy | True | True |
| `langs/lib_python/modules/textwrap.py` | `Lib/textwrap.py` | copy | True | True |
| `langs/lib_python/modules/typing.py` | `Lib/typing.py` | adapter | False | True |
| `langs/lib_python/modules/weakref.py` | `Lib/weakref.py` | adapter | False | True |
| `langs/lib_python/modules/test/test_import/data/syntax_warnings.py` | `Lib/test/test_import/data/syntax_warnings.py` | copy | True | False |
| `langs/lib_python/modules/test/test_iter.py` | `Lib/test/test_iter.py` | copy | True | True |
| `langs/lib_python/modules/test/test_math.py` | `Lib/test/test_math.py` | adapter | False | True |
| `langs/lib_python/modules/test/typinganndata/__init__.py` | `Lib/test/typinganndata/__init__.py` | copy | True | True |
| `langs/lib_python/modules/test/typinganndata/ann_module.py` | `Lib/test/typinganndata/ann_module.py` | adapter | False | True |
| `langs/lib_python/modules/test/typinganndata/ann_module2.py` | `Lib/test/typinganndata/ann_module2.py` | adapter | False | True |
| `langs/lib_python/modules/test/typinganndata/ann_module3.py` | `Lib/test/typinganndata/ann_module3.py` | adapter | False | True |

## Complete scratch copies

These replay the same upstream test and are repinned with their probe comment retained.

| Local file | Upstream path |
|---|---|
| `scratch/file-builtin/28.py` | `Lib/test/test_builtin.py` |
| `scratch/file-iter/12.py` | `Lib/test/test_iter.py` |
| `scratch/file-iter/13.py` | `Lib/test/test_contains.py` |
| `scratch/file-iter/14.py` | `Lib/test/test_enumerate.py` |
| `scratch/file-iter/15.py` | `Lib/test/test_range.py` |
| `scratch/reader-tail/1.py` | `Lib/test/test_fstring.py` |
| `scratch/reader-tail/2.py` | `Lib/test/test_print.py` |
| `scratch/reader-tail/3.py` | `Lib/test/test_grammar.py` |
| `scratch/reader-tail/4.py` | `Lib/test/test_set.py` |
| `scratch/reader-tail/5.py` | `Lib/test/test_listcomps.py` |
| `scratch/reader-tail/6.py` | `Lib/test/test_funcattrs.py` |
| `scratch/reader-tail/7.py` | `Lib/test/test_opcodes.py` |
| `scratch/reader-tail/9.py` | `Lib/test/test_decorators.py` |
