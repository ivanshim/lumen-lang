# Unicode Character Database

The files in this directory are unmodified copies from Unicode, Inc.:

- `UnicodeData.txt`: https://www.unicode.org/Public/16.0.0/ucd/UnicodeData.txt
- `NameAliases.txt`: https://www.unicode.org/Public/16.0.0/ucd/NameAliases.txt
- `NamedSequences.txt`: https://www.unicode.org/Public/16.0.0/ucd/NamedSequences.txt
- `UnicodeData-3.2.0.txt`: https://www.unicode.org/Public/3.2-Update/UnicodeData-3.2.0.txt

`LICENSE.txt` is the Unicode license from
https://www.unicode.org/license.txt . The algorithmic names for Hangul
syllables, CJK unified ideographs, and Tangut ideographs follow Unicode
Standard Annex #44: https://www.unicode.org/reports/tr44/ . The 3.2.0
file backs `unicodedata.ucd_3_2_0`; the 16.0.0 files back the rest of
`unicodedata` and the `\N{...}` escapes.
Standard Annex #44: https://www.unicode.org/reports/tr44/ .

## Reproducible database and string tables

Run `python3 scripts/generate_unicodedata.py` from the repository root.
Official release inputs are cached under ignored `probe/ucd/`. The committed
`generated/sources.json` records each original URL and SHA-256; regeneration
rejects a changed cached input. `generated/` contains compact property ranges,
canonical names, aliases/sequences, decomposition and composition mappings.
The script also regenerates each kernel's existing predicate and full-case
mapping tables in its own representation. Unihan supplies CJK numeric values;
DerivedCoreProperties, SpecialCasing and CaseFolding supply string behavior.

The 3.2 view follows CPython's change-record semantics: category, bidi,
decimal, numeric, mirrored and width retain historical values; digit remains
current. Assigned characters use current combining classes and public
decompositions. Normalization preserves unassigned characters and applies the
five historical normalization corrections. Lookup accepts current canonical
names even in the old view, but aliases and named sequences are unavailable
there. Tangut algorithmic names are lookup-only in CPython 3.14.8.

The private `__sre_native` bridge uses operation 18 to bind native database
methods to a property index and version. Queries 0–12 are category, bidi,
combining, mirrored, width, decimal, digit, numeric, decomposition, name,
lookup, normalize and is_normalized. Native callables validate argument counts
and character/string receivers before reading data, and preserve explicit
default objects unchanged. Existing bridge operations remain available.
