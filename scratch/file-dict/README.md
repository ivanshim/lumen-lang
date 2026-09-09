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
leaves class declarations, unparenthesized tuple commas, `with`, `del`, `yield`, and
several operators unread. No class body is passed over here, and no
new label is supplied for a construct belonging to those pieces.

The first Actions run on the given base reports `Unexpected character
'&'` for `test_dict.py` and `test_set.py`, and `Unexpected token: :`
for `test_dictcomps.py` and `test_setcomps.py`, on both kernels. The
Python reasons table counts six files at the former complaint and
21 at the latter. The spelling of `ext.op.bit.and` belongs to the
lexical piece, and `ext.stmt.class` to the class piece. The full-file reading has not been proved here.
