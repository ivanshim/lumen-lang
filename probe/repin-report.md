# CPython v3.14.8 repin measurements

Base: `98fceac01` (batch 20w); branch: `fix/repin-314`. Target: `v3.14.8`, `8e6e75d9102e`, 2026-09-30. `HISTORY.md` is unchanged; the coordinator supplies the historical correction.

Inventory: 120 entries (107 primary files plus 13 full scratch copies). 105 release bodies verified with SHA-256: 72 tests/support files, 20 complete library sources, 13 scratch copies. 58 bodies changed; 47 already matched. Thirteen separately documented partial runtime adapters retain their status, with provenance and release-required behavior updated. See `inventory.md`, `inventory.json`, `scratch-copies.json`, `replaced.json`, and `sha256.txt` for every path and upstream mapping.

Removed only `tests/python/test_import/data/syntax_warnings.py` and `langs/lib_python/modules/test/test_import/data/syntax_warnings.py`: `Lib/test/test_import/data/syntax_warnings.py` has no v3.14.8 counterpart. No top-level reference test file was removed. No missing release dependency required adding a CPython file. Added six runtime bridge files: `_operator.py` and `runtime_adapters/{copy,copyreg,functools,heapq,operator}.py`; complete upstream library bodies remain unchanged beneath their provenance headers. Added the focused `scratch/repin-314/1.py` probe and its output record.

## Language level

Python now reports 3.14.8 through sys.version, sys.version_info, sys.hexversion, and platform.python_version. Removed post-release sentinel/frozendict and twelve math APIs, hid gi_state, disabled lazy imports and iterator stop keywords in the Python definition, and rejected starred comprehensions. A relative module actually named lazy remains legal. Other language definitions retain their features. Duplicate-argument, mapping-pattern, and percent-format errors use release wording. Native constructor keys, bound method receivers, argument expansion, cyclic deepcopy identity, heap iterator bridges, operator.index, and integer math conversions were repaired generally behind Python labels or in Python runtime adapters. Ordered unittest assertions now construct failure messages only when the comparison fails, preserving huge-integer passing assertions.

## Every reference file

Values are pass/ran. The two kernel columns independently agree for every file. Skips count toward ran. All 60 executable reference files were measured; supporting files are inventoried but are not separate test programs.

| File | stack8 before → after | microcode7 before → after |
|---|---|---|
| `test_augassign` | 7/7 → 7/7 | 7/7 → 7/7 |
| `test_bigmem` | 60/61 → 60/61 | 60/61 → 60/61 |
| `test_binop` | 12/12 → 12/12 | 12/12 → 12/12 |
| `test_bisect` | 46/46 → 46/46 | 46/46 → 46/46 |
| `test_bool` | 31/31 → 31/31 | 31/31 → 31/31 |
| `test_builtin` | 117/143 → 108/133 | 117/143 → 108/133 |
| `test_class` | 40/42 → 41/42 | 40/42 → 41/42 |
| `test_cmath` | 32/33 → 32/33 | 32/33 → 32/33 |
| `test_compare` | 16/16 → 16/16 | 16/16 → 16/16 |
| `test_complex` | 37/37 → 37/38 | 37/37 → 37/38 |
| `test_contains` | 4/4 → 4/4 | 4/4 → 4/4 |
| `test_copy` | 83/83 → 81/81 | 83/83 → 81/81 |
| `test_decorators` | 16/16 → 16/16 | 16/16 → 16/16 |
| `test_dict` | 131/142 → 111/122 | 131/142 → 111/122 |
| `test_dictcomps` | 11/11 → 10/10 | 11/11 → 10/10 |
| `test_enumerate` | 77/105 → 77/105 | 77/105 → 77/105 |
| `test_eof` | 6/6 → 6/6 | 6/6 → 6/6 |
| `test_exceptions` | 102/115 → 96/111 | 102/115 → 96/111 |
| `test_float` | 50/54 → 50/54 | 50/54 → 50/54 |
| `test_fnmatch` | 24/24 → 24/24 | 24/24 → 24/24 |
| `test_format` | 16/18 → 16/19 | 16/18 → 16/19 |
| `test_fractions` | 50/50 → 50/50 | 50/50 → 50/50 |
| `test_fstring` | 91/92 → 91/94 | 91/92 → 91/94 |
| `test_funcattrs` | 38/39 → 37/38 | 38/39 → 37/38 |
| `test_functools` | 156/340 → 152/326 | 156/340 → 152/326 |
| `test_generators` | 56/59 → 54/59 | 56/59 → 54/59 |
| `test_genexps` | 1/1 → 1/1 | 1/1 → 1/1 |
| `test_global` | 20/20 → 20/20 | 20/20 → 20/20 |
| `test_grammar` | 75/75 → 75/75 | 75/75 → 75/75 |
| `test_heapq` | 69/69 → 69/69 | 69/69 → 69/69 |
| `test_index` | 55/55 → 55/55 | 55/55 → 55/55 |
| `test_int` | 40/52 → 40/52 | 40/52 → 40/52 |
| `test_int_literal` | 6/6 → 6/6 | 6/6 → 6/6 |
| `test_iter` | 65/68 → 55/57 | 65/68 → 55/57 |
| `test_itertools` | 111/136 → 111/136 | 111/136 → 111/136 |
| `test_keyword` | 11/11 → 11/11 | 11/11 → 11/11 |
| `test_keywordonlyarg` | 11/11 → 11/11 | 11/11 → 11/11 |
| `test_list` | 67/71 → 65/68 | 67/71 → 65/68 |
| `test_listcomps` | 68/68 → 66/66 | 68/68 → 66/66 |
| `test_long` | 40/47 → 40/47 | 40/47 → 40/47 |
| `test_math` | 84/88 → 83/88 | 84/88 → 83/88 |
| `test_opcodes` | 8/8 → 8/8 | 8/8 → 8/8 |
| `test_operator` | 49/110 → 106/112 | 49/110 → 106/112 |
| `test_positional_only_arg` | 27/28 → 27/28 | 27/28 → 27/28 |
| `test_pow` | 7/7 → 7/7 | 7/7 → 7/7 |
| `test_print` | 9/9 → 9/9 | 9/9 → 9/9 |
| `test_range` | 29/29 → 29/29 | 29/29 → 29/29 |
| `test_scope` | 38/41 → 38/41 | 38/41 → 38/41 |
| `test_set` | 644/644 → 630/630 | 644/644 → 630/630 |
| `test_setcomps` | 2/2 → 2/2 | 2/2 → 2/2 |
| `test_shlex` | 30/46 → 29/45 | 30/46 → 29/45 |
| `test_slice` | 11/11 → 11/11 | 11/11 → 11/11 |
| `test_str` | 127/138 → 127/138 | 127/138 → 127/138 |
| `test_string_literals` | 20/20 → 20/20 | 20/20 → 20/20 |
| `test_syntax` | 93/109 → 38/48 | 93/109 → 38/48 |
| `test_textwrap` | 68/68 → 68/68 | 68/68 → 68/68 |
| `test_tuple` | 34/38 → 34/38 | 34/38 → 34/38 |
| `test_unary` | 6/6 → 6/6 | 6/6 → 6/6 |
| `test_unpack` | 2/2 → 2/2 | 2/2 → 2/2 |
| `test_with` | 55/55 → 54/54 | 55/55 → 54/54 |

## Every formerly passing case that no longer passes

Identities, rather than ordinal positions, decide losses. Removed methods are separately classified; changed suite ordering is not a regression.

| File | Case | Result and classification |
|---|---|---|
| `test_builtin` | `BuiltinTest.test_all_any_tuple_list_set_optimization` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_eval_frozendict` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_exec_filter_syntax_warnings_by_module` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_exec_frozendict` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_sentinel` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_sentinel_attributes` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_sentinel_pickle` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_sentinel_repr` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `BuiltinTest.test_sentinel_str_subclass_name_cycle` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_builtin` | `TestType.test_type_frozendict` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_copy` | `TestCopy.test_copy_frozendict` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_copy` | `TestCopy.test_deepcopy_frozendict` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_bool` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_constructor` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_get` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_getitem` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_items` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_keys` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_len` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_read` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictMappingTests.test_values` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_constructor` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_copy` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_fromkeys` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_hash` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_items_xor` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_merge` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_pickle` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_pickle_iter` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_repr` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_unhashable_key` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dict` | `FrozenDictTests.test_update` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_dictcomps` | `DictComprehensionTest.test_hash_error` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_exceptions` | `AttributeErrorTests.test_class_getattr_error_message` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_exceptions` | `AttributeErrorTests.test_getattr_error_message` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_exceptions` | `AttributeErrorTests.test_module_getattr_error_message` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_exceptions` | `ImportErrorTests.test_ModuleNotFoundError_repr_with_failed_import` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_exceptions` | `ImportErrorTests.test_repr` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_exceptions` | `SyntaxErrorTests.test_encodings` | 3.14 asks differently; retained failing. Release expects a three-character caret span, whereas the development test expected the wider span currently produced by the simplified syntax highlighter. |
| `test_fstring` | `TestCase.test_double_brace_ast_location_covers_both_source_braces` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_funcattrs` | `BuiltinFunctionPropertiesTest.test_builtin__module__` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_functools` | `TestLRUPy.test_lru_checks_arg_is_callable` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_functools` | `TestReducePy.test_reduce_with_kwargs` | 3.14 asks differently; retained failing. Release accepts deprecated keyword arguments and checks warning attribution with skip_file_prefixes. The warnings adapter cannot attribute that warning through excluded filenames. |
| `test_functools` | `TestSingleDispatch.test_method_non_descriptor` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_functools` | `TestSingleDispatch.test_positional_only_argument` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_generators` | `ModifyUnderlyingIterableTest.test_modify_f_locals` | 3.14 asks differently; retained failing. Release expects the older generator-expression iterable/frame behavior; writable reflected generator locals are not implemented. |
| `test_generators` | `ModifyUnderlyingIterableTest.test_new_gen_from_gi_code` | 3.14 asks differently; retained failing. Release constructs a generator from gi_code with the older iterable convention. FunctionType reconstruction of generator-expression code is unsupported. |
| `test_iter` | `TestCase.test_calliter_reduce` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_calliter_setstate` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception_and_stop` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception_errors` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception_not_matching` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception_stop_iteration` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception_stop_iteration_leak` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_exception_tuple` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_iter` | `TestCase.test_iter_keyword_stop` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_list` | `ListTest.test_equal_operator_modifying_operand` | 3.14 asks differently; retained failing. Release expects equality after both operands are cleared inside __eq__ returning NotImplemented; the current comparison retains pre-mutation entries. |
| `test_list` | `ListTest.test_richcompare_stale_element_list_vitem` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_list` | `ListTest.test_richcompare_stale_element_list_witem` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_listcomps` | `ListComprehensionTest.test_async_optimization_with_side_effects` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_listcomps` | `ListComprehensionTest.test_optimization_with_side_effects` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_listcomps` | `ListComprehensionTest.test_optimization_with_starred_unpack` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `FMATests.test_fma_random` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.testIsnormal` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.testIssubnormal` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.test_fmax` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.test_fmax_nans` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.test_fmin` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.test_fmin_nans` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_math` | `MathTests.test_signbit` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_difference` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_difference_update` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_difference_update_operator` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_eq_ne` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_ge_gt_le_lt` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_intersection` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_intersection_update` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_intersection_update_operator` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_sym_difference` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_sym_difference_update` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_sym_difference_update_operator` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_union` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_update` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_set` | `TestOnlySetsFrozenDict.test_update_operator` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_setcomps` | `SetComprehensionTest.test_hash_error` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_shlex` | `ShlexTest.testForceQuote` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_assign_call` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_assign_del` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_bad_outdent` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_barry_as_flufl_with_syntax_errors` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_break_outside_loop` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_case_call_does_not_raise_syntax_error` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_continuation_bad_indentation` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_continue_outside_loop` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_curly_brace_after_primary_raises_immediately` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_double_ampersand` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_double_pipe` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_empty_line_after_linecont` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_error_parenthesis` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_error_string_literal` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_except_star_then_except` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_except_stmt_invalid_as_expr` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_except_then_except_star` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_expression_with_assignment` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_generator_in_function_call` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_global_param_err_first` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_ifexp_body_stmt_else_expression` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_ifexp_body_stmt_else_stmt` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_ifexp_else_stmt` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_invalid_line_continuation_error_position` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_invalid_line_continuation_left_recursive` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_invisible_characters` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_kwargs_last` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_kwargs_last2` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_kwargs_last3` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_in_async_function` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_in_class` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_in_except_block` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_in_function` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_in_try_block` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_in_trystar_block` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_nested_scopes` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_star_forbidden` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_lazy_import_valid_cases` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_match_call_does_not_raise_syntax_error` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_match_stmt_invalid_as_expr` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_multiline_compiler_error_points_to_the_end` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_multiline_string_concat_missing_comma_points_to_last_string` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_no_indent` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_nonlocal_param_err_first` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_raise_from_error_message` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_return_outside_function` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_unexpected_indent` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `LazyImportRestrictionTestCase.test_yield_outside_function` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxErrorTestCase.test_double_ampersand` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxErrorTestCase.test_double_pipe` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxErrorTestCase.test_multiline_string_concat_missing_comma_points_to_last_string` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxErrorTestCase.test_raise_from_error_message` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxWarningTest.test_from_lazy_imports` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxWarningTest.test_from_lazy_imports_as_error` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_syntax` | `SyntaxWarningTest.test_not_from_lazy_imports` | Removed upstream in v3.14.8; no retained test now fails. |
| `test_with` | `InterruptDuringEnter.test_exit_called_after_interrupt` | Removed upstream in v3.14.8; no retained test now fails. |

## Failures introduced during replacement and fixed

Regression fixed: TestCopy.test_deepcopy_bound_method, test_deepcopy_dict_subclass, test_deepcopy_inst_getinitargs, test_deepcopy_inst_getnewargs, test_deepcopy_inst_getnewargs_ex, test_deepcopy_inst_getstate, test_deepcopy_inst_getstate_setstate, test_deepcopy_inst_setstate, test_deepcopy_inst_vanilla, test_deepcopy_list_subclass, test_deepcopy_reflexive_dict, test_deepcopy_reflexive_inst, test_deepcopy_registry, test_deepcopy_slots, test_deepcopy_tuple_subclass, test_deepcopy_weakkeydict, test_deepcopy_weakvaluedict, and test_reconstruct_reflexive. The release deepcopy defaults and runtime identity/receiver adapters now preserve native reduction and cycles. Final copy: 81/81 both kernels.

Regression fixed: TestHeapC.test_merge_stability and TestHeapPython.test_merge_stability. Kept the existing native iterator bridge separately from upstream heapq; final heap: 69/69 including the merge doctest, both kernels.

Regression fixed: COperatorTestCase.test___all__ and PyOperatorTestCase.test___all__. The separate accelerator exposes its release-compatible export list; final operator: 106/112, six skips, both kernels.

Regression fixed: TestBasicOps.test_filter (general generator/custom iterator star-call expansion), ImportErrorTests.test_copy_pickle (release deepcopy adapter), MathTests.testComb, MathTests.testFactorialNonIntegers, MathTests.testPerm, and MathTests.testIsqrt (operator.index/type ordering and lazy assertion messages). Each recovered on both kernels.

3.14 asks differently, fixed: ExceptionTests.testSyntaxErrorOffset and the test_syntax module doctests through release mapping-pattern positions/wording and starred-comprehension rejection; StrTest.test_formatting through release integer percent-format wording.

## New release tests that fail

- `test_dict: DictTest.test_reversed_dict_keys_changed_during_iteration`: F; unsupported dictionary mutation iteration, non-UTF-8 source offsets, or diamond/future syntax diagnostic behavior, respectively.
- `test_exceptions: ExceptionTests.testSyntaxErrorNonUTF8Offset`: F; unsupported dictionary mutation iteration, non-UTF-8 source offsets, or diamond/future syntax diagnostic behavior, respectively.
- `test_syntax: SyntaxErrorTestCase.test_diamond_operator`: F; unsupported dictionary mutation iteration, non-UTF-8 source offsets, or diamond/future syntax diagnostic behavior, respectively.
- `test_syntax: SyntaxErrorTestCase.test_diamond_operator_barry_as_flufl`: E; unsupported dictionary mutation iteration, non-UTF-8 source offsets, or diamond/future syntax diagnostic behavior, respectively.

The existing f-string bytecode inspection case remains an error; the two added environment-dependent f-string cases are skipped. No retained passing test silently became skipped. The release adds/skips other math tests; totals change accordingly.

## Scratch records

Re-derived only: `scratch/file-builtin/28.err` (133 tests), `scratch/reader-tail/1.err` (94 tests), `scratch/modules-3/6.out` (3.14.8), `scratch/expressions/{3,6,10,11}.err` and `scratch/format-spec/4.err` (release formatting wording), `scratch/integrate-20f/1.out` → `.err` (removed iterator keyword), `scratch/comprehensions/11.out` → `.err`, and `scratch/file-list/11.out` → `.err` (post-release comprehension syntax). Each was measured on both kernels and written using stderr_record.measured_line, including its first-line traceback rule. The thirteen copied scratch sources are also verified against upstream. `reader-tail/4` was never run on the box; its release copy retains the empty output record and passed the official Lambda sweep.

## Checks and limitations

One official, unmodified full checker invocation: `checked 1181 programs, 0 mismatches`; gates `lumen 522/522, php 318/318, python 128/128`. Its snapshot was commit 960fd4bff with three documentation changes, before later fixes, the two starred-comprehension record changes, and the four numeric-format record changes. The checker printed expected count differences, five retained release-semantic failures, transient adapter/wording failures subsequently repaired, and Lambda timeouts. It is not a certification of the final HEAD; the requested single full invocation was not repeated. Later focused measurements, scratch verification, and gates validate the subsequent changes.

Lambda split runs recovered heap/range/itertools/long results. The single MathTests.testFsum method exceeds Lambda’s approximately 870-second cap even alone; before and after, both kernels completed that isolated method on the box in 1,163–1,233 seconds. No whole reference file was run on the box. Final math includes the measured testFsum result and post-fix testIsqrt result. The baseline packaged debug binary was used without changing the worktree. The measurement-only old heap selector supplies DocTestCase.class_ so the old selector can address its doctest; assertions and upstream source remain unchanged. `counts-final.json` records the origin of every count.

Final warnings-as-errors debug build: clean. Kernel independence: 0 problem(s). port_examples.py and lang_table.py: idempotent, no resulting diff. Fresh final-build example gates: lumen 522/522, php 318/318, python 128/128. Source verification: 105 SHA-256 matches. Tests/REPORT.md replayed complete Python measurements while retaining every unchanged PHP status/reason; CI’s removed pass/pass-row check reports none. All commits are signed. No push or PR.
