#!/usr/bin/env python3
"""Generate the two UCD databases from official Unicode release inputs.

Run from the repository root. Downloads stay in the ignored probe/ucd cache;
source URLs and SHA-256 hashes accompany the generated TSV tables. No host
unicodedata module supplies values. The 3.2 view follows CPython's historical
change-record rules (current digit, combining and decomposition for assigned
characters, old decimal/numeric/category/bidi/mirrored/width).
"""
import hashlib
import json
from pathlib import Path
from fractions import Fraction
from urllib.request import urlopen
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / 'probe/ucd'
OUT = ROOT / 'unicode-data/generated'
LIMIT = 0x110000
sources = {}

# Download and fingerprint an official input, retaining the original bytes.
def source(version, filename):
    if version == '16.0.0':
        url = f'https://www.unicode.org/Public/{version}/ucd/{filename}'
    else:
        stem, suffix = filename.rsplit('.', 1)
        url = f'https://www.unicode.org/Public/3.2-Update/{stem}-3.2.0.{suffix}'
    path = CACHE / version / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        path.write_bytes(urlopen(url, timeout=90).read())
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    manifest = OUT / 'sources.json'
    if manifest.exists():
        previous = json.loads(manifest.read_text()).get(f'{version}/{filename}')
        if previous and previous['sha256'] != digest:
            raise ValueError(f'Unicode source hash mismatch: {version}/{filename}')
    sources[f'{version}/{filename}'] = {'url': url, 'sha256': digest}
    return path

# Read semicolon records with comments and blank lines removed.
def records(path):
    for line in path.read_text(encoding='utf-8-sig').splitlines():
        body = line.split('#')[0].strip()
        if body:
            yield [cell.strip() for cell in body.split(';')]

# Expand the hexadecimal range syntax used by UCD property files.
def points(field):
    ends = field.split('..')
    first, last = int(ends[0], 16), int(ends[-1], 16)
    return range(first, last + 1)

# Expand UnicodeData's First/Last records and read Unihan numeric values.
def load(version):
    table = [None] * LIMIT
    first = None
    for row in records(source(version, 'UnicodeData.txt')):
        cp = int(row[0], 16)
        if row[1].endswith(', First>'):
            first = cp, row
        elif row[1].endswith(', Last>'):
            start, template = first
            for n in range(start, cp + 1):
                table[n] = template.copy()
            first = None
        else:
            table[cp] = row
    with ZipFile(source(version, 'Unihan.zip')) as archive:
        member = 'Unihan_NumericValues.txt' if version == '16.0.0' else 'Unihan-3.2.0.txt'
        for line in archive.read(member).decode('utf-8').splitlines():
            if not line.startswith('U+'):
                continue
            code, tag, value = line.split(None, 3)[:3]
            if tag in ('kAccountingNumeric', 'kPrimaryNumeric', 'kOtherNumeric'):
                cp = int(code[2:], 16)
                if table[cp] is not None:
                    table[cp][8] = value.replace(',', '')
    widths = ['N'] * LIMIT
    for field, width in records(source(version, 'EastAsianWidth.txt')):
        for cp in points(field):
            widths[cp] = width
    exclusions = set()
    for row in records(source(version, 'CompositionExclusions.txt')):
        exclusions.update(points(row[0]))
    return table, widths, exclusions

# Spell current names, including the Hangul and CJK algorithms.
def names(table):
    lead = 'G GG N D DD R M B BB S SS - J JJ C K T P H'.split()
    vowel = 'A AE YA YAE EO E YEO YE O WA WAE OE YO U WEO WE WI YU EU YI I'.split()
    tail = '- G GG GS N NJ NH D L LG LM LB LS LT LP LH M B BS S SS NG J C K T P H'.split()
    for cp, row in enumerate(table):
        if row is None:
            continue
        label = row[1]
        if 0xAC00 <= cp <= 0xD7A3:
            offset = cp - 0xAC00
            syllable = lead[offset // 588] + vowel[offset % 588 // 28] + tail[offset % 28]
            label = 'HANGUL SYLLABLE ' + syllable.replace('-', '')
        elif label.startswith('<CJK Ideograph'):
            label = f'CJK UNIFIED IDEOGRAPH-{cp:04X}'
        elif label.startswith('<'):
            continue
        yield cp, label

# Write contiguous identical properties as ranges for fast native lookup.
def write_properties(filename, values):
    with (OUT / filename).open('w') as stream:
        start, previous = 0, values[0]
        for cp in range(1, LIMIT + 1):
            current = values[cp] if cp < LIMIT else None
            if current != previous:
                stream.write(f'{start:X}\t{cp-1:X}\t' + '\t'.join(previous) + '\n')
                start, previous = cp, current

# Build property records with CPython's old-version compatibility semantics.
def properties(current, historic, widths, old_widths, old):
    result = []
    for cp, row in enumerate(current):
        selected = historic[cp] if old else row
        if selected is None:
            category, bidi, combining, mirror, decimal, numeric = 'Cn', '', '0', '0', '', ''
        else:
            category, bidi = selected[2], selected[4]
            combining = row[3]
            mirror, decimal, numeric = selected[9] == 'Y', selected[6], selected[8]
            mirror = str(int(mirror))
        width = ('N' if row is not None else widths[cp]) if selected is None else (old_widths[cp] if old else widths[cp])
        digit = row[7] if row else ''
        # Retain rationals exactly; Rust converts to a double at the boundary.
        if numeric:
            numeric = str(Fraction(numeric))
        result.append((category, bidi, combining, mirror, width, decimal, digit, numeric))
    return result

# Generate normalization mappings and canonical composition pairs.
def normalization(current, historic, exclusions):
    full_exclusions = set(exclusions)
    for row in records(source('16.0.0', 'DerivedNormalizationProps.txt')):
        if row[1] == 'Full_Composition_Exclusion':
            full_exclusions.update(points(row[0]))
    joins = []
    for cp, row in enumerate(current):
        if row and row[5] and not row[5].startswith('<'):
            pieces = [int(n, 16) for n in row[5].split()]
            if len(pieces) == 2 and cp not in full_exclusions:
                assert current[pieces[0]][3] == '0'
                joins.append((*pieces, cp))
    (OUT / 'composition.tsv').write_text(''.join(f'{a:X}\t{b:X}\t{c:X}\n' for a, b, c in sorted(joins)))
    for old, filename in ((False, 'decomposition.tsv'), (True, 'decomposition-3.2.tsv')):
        lines = []
        for cp, row in enumerate(current):
            if row and row[5] and (not old or historic[cp]):
                # decomposition() exposes the current mapping even in the old view.
                lines.append(f'{cp:X}\t{row[5]}\n')
        (OUT / filename).write_text(''.join(lines))
    corrections = []
    for cp, row in enumerate(current):
        prior = historic[cp]
        if row and prior and row[5] != prior[5]:
            assert prior[5] and ' ' not in prior[5]
            corrections.append(f'{cp:X}\t{prior[5]}\n')
    assert len(corrections) == 5
    (OUT / 'normalization-3.2.tsv').write_text(''.join(corrections))

# Generate predicates and full case mappings for the shared lumen-unicode crate.
def string_tables(current):
    masks = [0] * LIMIT
    for cp, row in enumerate(current):
        if row:
            category = row[2]
            if cp == 0x20 or category[0] not in 'CZ':
                masks[cp] |= 1
            if category in ('Lu', 'Ll', 'Lt', 'Lm', 'Lo'):
                masks[cp] |= 2048
            if row[6]:
                masks[cp] |= 2
            if row[7]:
                masks[cp] |= 1024
            if row[8]:
                masks[cp] |= 512
            if category == 'Lt':
                masks[cp] |= 256
    flags = {'XID_Start': 4, 'XID_Continue': 8, 'Cased': 16,
             'Case_Ignorable': 32, 'Uppercase': 64, 'Lowercase': 128}
    for row in records(source('16.0.0', 'DerivedCoreProperties.txt')):
        flag = flags.get(row[1], 0)
        if flag:
            for cp in points(row[0]):
                if current[cp]:
                    masks[cp] |= flag
    casing = {}
    for cp, row in enumerate(current):
        if row:
            lower, upper, title = (row[13] or f'{cp:X}', row[12] or f'{cp:X}', row[14] or row[12] or f'{cp:X}')
            casing[cp] = [lower, upper, title, f"{cp:X}"]
    for row in records(source('16.0.0', 'SpecialCasing.txt')):
        if not row[4]:
            cp = int(row[0], 16)
            casing[cp][:3] = [row[1], row[3], row[2]]
    for row in records(source('16.0.0', 'CaseFolding.txt')):
        if row[1] in ('C', 'F'):
            casing[int(row[0], 16)][3] = row[2]
    changes = []
    for cp, mappings in sorted(casing.items()):
        sequences = [[int(hex, 16) for hex in mapping.split()] for mapping in mappings]
        if any(sequence != [cp] for sequence in sequences):
            strings = ['"' + ''.join(f'\\u{{{n:x}}}' for n in sequence) + '"' for sequence in sequences]
            changes.append((cp, strings))
    runs = []
    start, last_mask = 0, masks[0]
    for cp in range(1, LIMIT + 1):
        flag = masks[cp] if cp < LIMIT else None
        if flag != last_mask:
            if last_mask:
                runs.append((start, cp - 1, last_mask))
            start, last_mask = cp, flag
    body = 'static RANGES: &[u64] = &[\n'
    packed = [f'0x{a << 37 | b << 16 | mask:x}' for a, b, mask in runs]
    body += ''.join('    ' + ', '.join(packed[i:i+6]) + ',\n' for i in range(0, len(packed), 6)) + '];\n\n'
    body += 'static CASES: &[(u32, [&str; 4])] = &[\n'
    body += ''.join(f'    (0x{cp:x}, [{", ".join(mapping)}]),\n' for cp, mapping in changes) + '];'
    path = ROOT / 'native' / 'unicode' / 'lib.rs'
    text = path.read_text()
    begin = text.index('static RANGES:')
    end = text.index('\n];', text.index('static CASES:')) + len('\n];')
    path.write_text(text[:begin] + body + text[end:])

# Produce deterministic files and verify structural invariants.
def main():
    OUT.mkdir(parents=True, exist_ok=True)
    current, widths, exclusions = load('16.0.0')
    historic, old_widths, old_exclusions = load('3.2.0')
    assert exclusions == old_exclusions
    for old, filename in ((False, 'properties.tsv'), (True, 'properties-3.2.tsv')):
        values = properties(current, historic, widths, old_widths, old)
        assert values[ord('A')][:2] == ('Lu', 'L')
        assert values[0xD800][0] == 'Cs'
        assert values[0x10FFFF][0] == 'Cn'
        write_properties(filename, values)
    normalization(current, historic, exclusions)
    string_tables(current)
    all_names = list(names(current))
    (OUT / 'names.tsv').write_text(''.join(f'{cp:X}\t{label}\n' for cp, label in all_names))
    lookup_only = [(cp, f'TANGUT IDEOGRAPH-{cp:04X}') for cp, row in enumerate(current) if row and row[1].startswith('<Tangut Ideograph')]
    (OUT / 'lookup-only.tsv').write_text(''.join(f'{label}\t{cp:X}\n' for cp, label in lookup_only))
    aliases = []
    for row in records(source('16.0.0', 'NameAliases.txt')):
        aliases.append(f'{row[1]}\t{row[0]}\n')
    for row in records(source('16.0.0', 'NamedSequences.txt')):
        aliases.append(f'{row[0]}\t{row[1]}\n')
    (OUT / 'aliases.tsv').write_text(''.join(aliases))
    (OUT / 'sources.json').write_text(json.dumps(sources, indent=2, sort_keys=True) + '\n')
    print(f'{len(all_names)} names; generated Unicode 16.0.0 and 3.2.0 databases')

if __name__ == '__main__':
    main()
