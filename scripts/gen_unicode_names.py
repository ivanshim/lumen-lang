#!/usr/bin/env python3
"""Generate the Python Unicode name tables from the Unicode Character Database.

Reads the release data files kept under ``unicode-data/`` and writes the
compact paged name modules that ``unicodedata`` and the text codecs read:

  * ``langs/lib_python/modules/_codec_names.py`` from ``UnicodeData.txt``,
    ``NameAliases.txt`` and ``NamedSequences.txt`` (Unicode 16.0.0);
  * ``langs/lib_python/modules/_ucd_3_2_0.py`` from
    ``UnicodeData-3.2.0.txt``.

Each character name is stored once, paged by the high byte of its code
point, so the runtime does not build one dictionary per character.  The
algorithmic Hangul syllable names and the CJK and Tangut ideograph ranges
are computed instead of stored.  Run from the repository root:

    python3 scripts/gen_unicode_names.py

Every generated file begins with a marker line; the script refuses to
overwrite a file without it.
"""

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UNICODE = os.path.join(ROOT, "unicode-data")
MODULES = os.path.join(ROOT, "langs", "lib_python", "modules")

LEADS = ("G", "GG", "N", "D", "DD", "R", "M", "B", "BB", "S", "SS", "", "J",
         "JJ", "C", "K", "T", "P", "H")
VOWELS = ("A", "AE", "YA", "YAE", "EO", "E", "YEO", "YE", "O", "WA", "WAE",
          "OE", "YO", "U", "WEO", "WE", "WI", "YU", "EU", "YI", "I")
TAILS = ("", "G", "GG", "GS", "N", "NJ", "NH", "D", "L", "LG", "LM", "LB",
         "LS", "LT", "LP", "LH", "M", "B", "BS", "S", "SS", "NG", "J", "C",
         "K", "T", "P", "H")

# label prefix -> (name prefix, whether name() reports it).  CPython's
# name() answers for the CJK ideographs but reaches the Tangut ideographs
# only through lookup(), so the two are kept apart.
RANGE_PREFIX = (
    ("<CJK Ideograph", "CJK UNIFIED IDEOGRAPH", True),
    ("<Tangut Ideograph", "TANGUT IDEOGRAPH", False),
)


def hangul_name(number):
    offset = number - 0xAC00
    return "HANGUL SYLLABLE " + LEADS[offset // 588] \
        + VOWELS[(offset % 588) // 28] + TAILS[offset % 28]


def read_unicode_data(path):
    """Return (explicit, ranges) for one UnicodeData.txt file."""
    explicit = {}
    ranges = []
    first = None
    with open(path, encoding="utf-8") as handle:
        for row in handle:
            row = row.rstrip("\n")
            if not row:
                continue
            cells = row.split(";")
            number = int(cells[0], 16)
            label = cells[1]
            if label.endswith(", First>"):
                first = (number, label)
            elif label.endswith(", Last>"):
                if first is None:
                    raise ValueError("range end without start in " + path)
                ranges.append((first[0], number, first[1]))
                first = None
            elif label.startswith("<"):
                continue
            else:
                explicit[number] = label
    if first is not None:
        raise ValueError("unfinished range in " + path)
    return explicit, ranges


def name_for(number, explicit, ranges):
    if 0xAC00 <= number <= 0xD7A3:
        return hangul_name(number)
    answer = explicit.get(number)
    if answer is not None:
        return answer
    for start, end, prefix, shown in ranges:
        if shown and start <= number <= end:
            return "%s-%04X" % (prefix, number)
    return None


def algorithmic_ranges(ranges):
    """Keep only the ranges that produce names, in prefix form."""
    kept = []
    for start, end, label in ranges:
        for marker, prefix, shown in RANGE_PREFIX:
            if label.startswith(marker):
                kept.append((start, end, prefix, shown))
                break
    kept.sort()
    return kept


def paged(explicit):
    pages = {}
    for number, label in explicit.items():
        pages.setdefault(number >> 8, []).append((number & 0xFF, label))
    for page in pages:
        pages[page].sort()
    return pages


def render_pages(paged_map):
    rows = []
    for page in sorted(paged_map):
        body = "".join("%02x:%s\\n" % (index, label)
                       for index, label in paged_map[page])
        rows.append("    %d: '\\n%s'," % (page, body))
    return "\n".join(rows)


def render_ranges(ranges):
    return ",\n".join(
        "    (0x%04X, 0x%04X, %r, %r)" % row for row in ranges)


def parse_aliases(path):
    aliases = {}
    with open(path, encoding="utf-8") as handle:
        for row in handle:
            row = row.rstrip("\n")
            if not row or row.startswith("#"):
                continue
            cells = row.split(";")
            if len(cells) < 2:
                continue
            aliases[cells[1]] = chr(int(cells[0], 16))
    return aliases


def parse_sequences(path):
    sequences = {}
    with open(path, encoding="utf-8") as handle:
        for row in handle:
            row = row.rstrip("\n")
            if not row or row.startswith("#"):
                continue
            title, points = row.split(";")
            sequences[title] = "".join(
                chr(int(point, 16)) for point in points.split())
    return sequences


def literal(text):
    return repr(text)


def write(path, body, marker):
    if os.path.exists(path):
        with open(path, encoding="utf-8") as handle:
            head = handle.readline()
        if marker not in head:
            raise SystemExit("refusing to overwrite " + path)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(body)
    print("wrote %s (%d bytes)" % (path, os.path.getsize(path)))


def main():
    explicit, ranges = read_unicode_data(
        os.path.join(UNICODE, "UnicodeData.txt"))
    aliases = parse_aliases(os.path.join(UNICODE, "NameAliases.txt"))
    sequences = parse_sequences(
        os.path.join(UNICODE, "NamedSequences.txt"))
    pages = paged(explicit)
    kept = algorithmic_ranges(ranges)

    alias_rows = "\n".join("    %s: %s," % (literal(key), literal(value))
                           for key, value in sorted(aliases.items()))
    sequence_rows = "\n".join("    %s: %s," % (literal(key), literal(value))
                              for key, value in sorted(sequences.items()))

    body = '''# Generated by scripts/gen_unicode_names.py from the Unicode 16.0.0
# Character Database; do not edit by hand.
unidata_version = '16.0.0'
_leads = %(leads)r
_vowels = %(vowels)r
_tails = %(tails)r
_ranges = [
%(ranges)s
]


_forward = None
_reverse = None


def _load():
    global _forward, _reverse
    if _forward is not None:
        return
    forward = {}
    reverse = {}
    for page, block in _names.items():
        for row in block.split('\\n'):
            if not row:
                continue
            at = row.find(':')
            number = (page << 8) + int(row[:at], 16)
            label = row[at + 1:]
            forward[number] = label
            reverse[label] = chr(number)
    _forward = forward
    _reverse = reverse


def name(number):
    global _forward
    if _forward is None:
        _load()
    if 0xac00 <= number <= 0xd7a3:
        offset = number - 0xac00
        return 'HANGUL SYLLABLE ' + _leads[offset // 588] \\
            + _vowels[offset %% 588 // 28] + _tails[offset %% 28]
    try:
        found = _forward[number]
    except KeyError:
        found = None
    if found is not None:
        return found
    for start, end, prefix, shown in _ranges:
        if shown and start <= number <= end:
            return '%%s-%%04X' %% (prefix, number)
    return None


def _hangul(spelling):
    head = 'HANGUL SYLLABLE '
    if not spelling.startswith(head):
        return None
    body = spelling[len(head):]
    for li in range(len(_leads)):
        if not body.startswith(_leads[li]):
            continue
        rest = body[len(_leads[li]):]
        for vi in range(len(_vowels)):
            if not rest.startswith(_vowels[vi]):
                continue
            tail = rest[len(_vowels[vi]):]
            for ti in range(len(_tails)):
                if tail == _tails[ti]:
                    number = 0xac00 + (li * 21 + vi) * 28 + ti
                    if name(number) == spelling:
                        return chr(number)
    return None


def lookup(spelling):
    global _reverse
    if _reverse is None:
        _load()
    spelling = spelling.upper()
    for start, end, prefix, shown in _ranges:
        head = prefix + '-'
        if spelling.startswith(head):
            tail = spelling[len(head):]
            try:
                number = int(tail, 16)
            except ValueError:
                continue
            if '%%04X' %% number != tail:
                continue
            if start <= number <= end:
                return chr(number)
            continue
    found = _hangul(spelling)
    if found is not None:
        return found
    try:
        found = _reverse[spelling]
    except KeyError:
        found = None
    if found is not None:
        return found
    try:
        return _aliases[spelling]
    except KeyError:
        raise KeyError('undefined character name ' + spelling)


def sequence(spelling):
    try:
        return _sequences[spelling.upper()]
    except KeyError:
        raise KeyError('undefined named sequence ' + spelling)


_names = {
%(pages)s
}

_aliases = {
%(aliases)s
}

_sequences = {
%(sequences)s
}
''' % {
        "leads": LEADS, "vowels": VOWELS, "tails": TAILS,
        "ranges": render_ranges(kept),
        "pages": render_pages(pages),
        "aliases": alias_rows, "sequences": sequence_rows,
    }
    write(os.path.join(MODULES, "_codec_names.py"), body,
          "Generated by scripts/gen_unicode_names.py")

    explicit, ranges = read_unicode_data(
        os.path.join(UNICODE, "UnicodeData-3.2.0.txt"))
    pages = paged(explicit)
    kept = algorithmic_ranges(ranges)
    body = '''# Generated by scripts/gen_unicode_names.py from the Unicode 3.2.0
# Character Database; do not edit by hand.
unidata_version = '3.2.0'
_leads = %(leads)r
_vowels = %(vowels)r
_tails = %(tails)r
_ranges = [
%(ranges)s
]


_forward = None
_reverse = None


def _load():
    global _forward, _reverse
    if _forward is not None:
        return
    forward = {}
    reverse = {}
    for page, block in _names.items():
        for row in block.split('\\n'):
            if not row:
                continue
            at = row.find(':')
            number = (page << 8) + int(row[:at], 16)
            label = row[at + 1:]
            forward[number] = label
            reverse[label] = chr(number)
    _forward = forward
    _reverse = reverse


def name(number):
    global _forward
    if _forward is None:
        _load()
    if 0xac00 <= number <= 0xd7a3:
        offset = number - 0xac00
        return 'HANGUL SYLLABLE ' + _leads[offset // 588] \\
            + _vowels[offset %% 588 // 28] + _tails[offset %% 28]
    try:
        found = _forward[number]
    except KeyError:
        found = None
    if found is not None:
        return found
    for start, end, prefix, shown in _ranges:
        if shown and start <= number <= end:
            return '%%s-%%04X' %% (prefix, number)
    return None


def _hangul(spelling):
    head = 'HANGUL SYLLABLE '
    if not spelling.startswith(head):
        return None
    body = spelling[len(head):]
    for li in range(len(_leads)):
        if not body.startswith(_leads[li]):
            continue
        rest = body[len(_leads[li]):]
        for vi in range(len(_vowels)):
            if not rest.startswith(_vowels[vi]):
                continue
            tail = rest[len(_vowels[vi]):]
            for ti in range(len(_tails)):
                if tail == _tails[ti]:
                    number = 0xac00 + (li * 21 + vi) * 28 + ti
                    if name(number) == spelling:
                        return chr(number)
    return None


def lookup(spelling):
    global _reverse
    if _reverse is None:
        _load()
    spelling = spelling.upper()
    for start, end, prefix, shown in _ranges:
        head = prefix + '-'
        if spelling.startswith(head):
            tail = spelling[len(head):]
            try:
                number = int(tail, 16)
            except ValueError:
                continue
            if '%%04X' %% number != tail:
                continue
            if start <= number <= end:
                return chr(number)
            continue
    found = _hangul(spelling)
    if found is not None:
        return found
    try:
        found = _reverse[spelling]
    except KeyError:
        found = None
    if found is not None:
        return found
    raise KeyError('undefined character name ' + spelling)


_names = {
%(pages)s
}
''' % {
        "leads": LEADS, "vowels": VOWELS, "tails": TAILS,
        "ranges": render_ranges(kept),
        "pages": render_pages(pages),
    }
    write(os.path.join(MODULES, "_ucd_3_2_0.py"), body,
          "Generated by scripts/gen_unicode_names.py")


if __name__ == "__main__":
    main()
