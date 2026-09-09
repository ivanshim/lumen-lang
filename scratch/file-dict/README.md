# Dictionary and set reading

`0.py` takes the tuple held by the set literal in
`tests/python/test_dict.py`, at line 115. Its answer is one. Both readers
formerly rejected its comma with `Expected ')' to close a group, got ','`.
`ext.op.tuple` now admits empty, singleton, nested and spread tuples inside
grouping marks. Like sets, tuples use the array representation at this
stage; tuple identity, immutability and rendering are not supplied here.
`5.py` checks these forms, grouping, indexing, and comprehension targets.

`1.py` asks about empty and nonempty brace literals, nested maps, and a
trailing separator across line ends. `syntax.map.*` supplies the map
marks and `ext.syntax.set` admits braces without pairs. These labels
were already spelled at the base. The questions yield whole numbers,
so they do not depend on the kernels' printed form for collections.

`2.py` takes the dictionary comprehensions from `test_dictcomps.py` and
the nested set comprehension from `test_setcomps.py`. It checks the
answer and the outer binding. `ext.op.comprehension.for`, `.in`, and
`.if` already supply these clauses; `ext.builtin.range.value` supplies
their whole-number walks. The set members in this probe are distinct,
so the earlier stage's array stand-in gives the same sum.

`3.py` gathers spread pairs and spread members, then nests one map
comprehension within another. `ext.syntax.map.spread` and
`ext.syntax.array.spread` already supply the marks. It asks that later
pairs replace earlier ones, and that each nested comprehension keep
its own name.

`4.py` reads the literal method calls from `test_dict.py` within a
routine. The routine is not called: this probe proves reading alone,
not the provision of dictionary methods. `op.pipe` and the existing
`ext.syntax.call.spread.pairs` supply the written forms.

The four whole reference files still need the earlier class,
block, expression, lexical, and string pieces. In particular, the base
leaves class declarations, `with`, `del`, `yield`, and
several operators unread. No class body is passed over here, and no
new label is supplied for a construct belonging to those pieces.

The first Actions run on the given base reports `Unexpected character
'&'` for `test_dict.py` and `test_set.py`, and `Unexpected token: :`
for `test_dictcomps.py` and `test_setcomps.py`, on both kernels. The
Python reasons table counts six files at the former complaint and
21 at the latter. The bit spellings are now enabled. The second green run advances all four
files to `Unexpected token: :` on both kernels. Class declarations remain
unread; no class body is skipped. Full-file reading has not been proved.

`6.py` spells the remaining bit operators used by dictionary and set view
expressions, using the kernels' existing operations and Python precedence.
Its executable checks use integers; the view calls are read within an
uncalled function. This removes the lexical rejection of `&` in the whole
files without claiming dictionary-view operations are implemented.

`7.py` checks ordinary loops over one-, two- and three-argument range values,
an empty range, and the tuple of sizes in `test_dict.py`. A language that
spells `ext.builtin.range.value` now walks that value in ordinary loops,
just as in comprehensions, instead of using the older two-bound range
syntax. Other languages retain that syntax.

`8.py` reads unparenthesized tuples after assignments, returns, and the
`in` of an ordinary loop, including the pair of mapping expressions in
`test_dict.py`. Commas inside calls still separate arguments.

`9.py` checks dictionary expansion into the print separator and ending
options, including null defaults, an empty argument list and positional
spread. `scratch/params/15.py` remains `print(1, end="")`; its old `.err`
expected a refusal of every builtin keyword argument. The replacement
`15.out` contains exactly the byte `1`, without a newline, because this
supported print option must behave as it does in CPython. Other unsupported
builtin keyword arguments still use the existing refusal.

`10.py` calls the dictionary constructor forms from `test_dict.py`: empty,
keyword, map copy and iterable pairs. Later pairs replace earlier ones;
keyword pairs replace those in the positional source, while duplicate
keyword arguments remain an error. The constructor walks the existing
collection values; user-defined mapping and iterator protocols remain open.

`14.py` checks the map merge operators from `test_dict.py`, including
right-hand replacement, insertion order, reversed operands and a compound
write. Integer bit-or remains available. The current map representation
uses value writes; shared mutable identity and the iterable-pair variant
of compound merge still need further work.

`11.py` through `13.py` check constructor refusals for duplicate keyword
names, pair length and positional argument count.

`15.py` unpacks tuple and map values into bare names, as the dictionary
suite does when setting up related mappings. The source is evaluated once
and its length checked before any target changes; swaps therefore read
both old values. Nested, starred and indexed assignment targets are not
covered by this bare-name path.

`16.py` compares maps built in different insertion orders, including
nested maps and maps inside arrays. The dictionary comparisons in the
reference suite require key/value equality, while iteration must retain
the original insertion order. Array element order still matters.
