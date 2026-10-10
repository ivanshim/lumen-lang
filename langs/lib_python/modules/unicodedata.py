# Unicode normalization and character properties from UCD 16.0.0 and 3.2.0.
# UAX #15 canonical decomposition, ordering, and composition; Hangul is algorithmic.
_unicode_mapping = __multibyte_native
import _codec_names
import _ucd_3_2_0
_no_default = object()

class UCD:
    def __init__(self, table, version):
        self._table = table
        self.unidata_version = version

    # Read names from the selected release without changing property/normalization data.
    def name(self, char, default=_no_default):
        if not isinstance(char, str) or len(char) != 1:
            raise TypeError('name() argument 1 must be a unicode character')
        table = _ucd_3_2_0 if self._table == 'ucd32' else _codec_names
        answer = table.name(ord(char))
        if answer is not None:
            return answer
        if default is not _no_default:
            return default
        raise ValueError('no such name')

    # Resolve scalar names in either release and modern named sequences in 16.0.
    def lookup(self, name):
        if not isinstance(name, str):
            raise TypeError('lookup() argument must be str')
        if self._table == 'ucd32':
            return _ucd_3_2_0.lookup(name)
        try:
            return _codec_names.lookup(name)
        except KeyError:
            return _codec_names.sequence(name)

    def _property(self, char):
        if not isinstance(char, str) or len(char) != 1:
            raise TypeError('argument must be a unicode character, not ' + type(char).__name__)
        return _unicode_mapping(4, self._table, ord(char), 0)

    def category(self, char):
        return self._property(char)[0]

    def bidirectional(self, char):
        return self._property(char)[1]

    def combining(self, char):
        return self._property(char)[2]

    def mirrored(self, char):
        return self._property(char)[3]

    def _expand(self, cp, compatible, result):
        if 0xAC00 <= cp <= 0xD7A3:
            offset = cp - 0xAC00
            result.append(0x1100 + offset // 588)
            result.append(0x1161 + (offset % 588) // 28)
            if offset % 28:
                result.append(0x11A7 + offset % 28)
            return
        entry = _unicode_mapping(5, self._table, cp, 0)
        if entry is not None and (compatible or not entry[0]):
            for part in entry[1]:
                self._expand(part, compatible, result)
        else:
            result.append(cp)

    def normalize(self, form, text):
        if not isinstance(form, str):
            raise TypeError('normalize() argument 1 must be str, not ' + type(form).__name__)
        if form not in ('NFD', 'NFC', 'NFKD', 'NFKC'):
            raise ValueError('invalid normalization form')
        if not isinstance(text, str):
            raise TypeError('normalize() argument 2 must be str, not ' + type(text).__name__)
        expanded = []
        for char in text:
            self._expand(ord(char), form in ('NFKD', 'NFKC'), expanded)
        ordered = []
        classes = []
        for cp in expanded:
            cc = self.combining(chr(cp))
            pos = len(ordered)
            if cc:
                while pos and classes[pos-1] > cc:
                    pos -= 1
            ordered.insert(pos, cp)
            classes.insert(pos, cc)
        if form in ('NFD', 'NFKD'):
            return ''.join(chr(cp) for cp in ordered)
        composed = []
        starter = -1
        last_cc = 0
        for cp, cc in zip(ordered, classes):
            joined = None
            if starter >= 0 and (last_cc < cc or last_cc == 0):
                first = composed[starter]
                if 0x1100 <= first <= 0x1112 and 0x1161 <= cp <= 0x1175:
                    joined = 0xAC00 + (first - 0x1100) * 588 + (cp - 0x1161) * 28
                elif 0xAC00 <= first <= 0xD7A3 and (first - 0xAC00) % 28 == 0 and 0x11A8 <= cp <= 0x11C2:
                    joined = first + cp - 0x11A7
                else:
                    joined = _unicode_mapping(6, self._table, first, cp)
            if joined is not None:
                composed[starter] = joined
            else:
                if cc == 0:
                    starter = len(composed)
                composed.append(cp)
                last_cc = cc
        return ''.join(chr(cp) for cp in composed)

    def is_normalized(self, form, text):
        return self.normalize(form, text) == text

ucd_3_2_0 = UCD('ucd32', '3.2.0')
_default = UCD('ucd16', '16.0.0')
unidata_version = _default.unidata_version
category = _default.category
bidirectional = _default.bidirectional
combining = _default.combining
mirrored = _default.mirrored
normalize = _default.normalize
is_normalized = _default.is_normalized

name = _default.name
lookup = _default.lookup

# Native decomposition retains the complete CPython descriptor contract.
decomposition = __unicode_decomposition
