# Unicode normalization and character properties from UCD 16.0.0 and 3.2.0.
# UAX #15 canonical decomposition, ordering, and composition; Hangul is algorithmic.
_unicode_mapping = __multibyte_native

class UCD:
    def __init__(self, table, version):
        self._table = table
        self.unidata_version = version

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

_names = ['', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', 'SPACE', 'EXCLAMATION MARK', 'QUOTATION MARK', 'NUMBER SIGN', 'DOLLAR SIGN', 'PERCENT SIGN', 'AMPERSAND', 'APOSTROPHE', 'LEFT PARENTHESIS', 'RIGHT PARENTHESIS', 'ASTERISK', 'PLUS SIGN', 'COMMA', 'HYPHEN-MINUS', 'FULL STOP', 'SOLIDUS', 'DIGIT ZERO', 'DIGIT ONE', 'DIGIT TWO', 'DIGIT THREE', 'DIGIT FOUR', 'DIGIT FIVE', 'DIGIT SIX', 'DIGIT SEVEN', 'DIGIT EIGHT', 'DIGIT NINE', 'COLON', 'SEMICOLON', 'LESS-THAN SIGN', 'EQUALS SIGN', 'GREATER-THAN SIGN', 'QUESTION MARK', 'COMMERCIAL AT', 'LATIN CAPITAL LETTER A', 'LATIN CAPITAL LETTER B', 'LATIN CAPITAL LETTER C', 'LATIN CAPITAL LETTER D', 'LATIN CAPITAL LETTER E', 'LATIN CAPITAL LETTER F', 'LATIN CAPITAL LETTER G', 'LATIN CAPITAL LETTER H', 'LATIN CAPITAL LETTER I', 'LATIN CAPITAL LETTER J', 'LATIN CAPITAL LETTER K', 'LATIN CAPITAL LETTER L', 'LATIN CAPITAL LETTER M', 'LATIN CAPITAL LETTER N', 'LATIN CAPITAL LETTER O', 'LATIN CAPITAL LETTER P', 'LATIN CAPITAL LETTER Q', 'LATIN CAPITAL LETTER R', 'LATIN CAPITAL LETTER S', 'LATIN CAPITAL LETTER T', 'LATIN CAPITAL LETTER U', 'LATIN CAPITAL LETTER V', 'LATIN CAPITAL LETTER W', 'LATIN CAPITAL LETTER X', 'LATIN CAPITAL LETTER Y', 'LATIN CAPITAL LETTER Z', 'LEFT SQUARE BRACKET', 'REVERSE SOLIDUS', 'RIGHT SQUARE BRACKET', 'CIRCUMFLEX ACCENT', 'LOW LINE', 'GRAVE ACCENT', 'LATIN SMALL LETTER A', 'LATIN SMALL LETTER B', 'LATIN SMALL LETTER C', 'LATIN SMALL LETTER D', 'LATIN SMALL LETTER E', 'LATIN SMALL LETTER F', 'LATIN SMALL LETTER G', 'LATIN SMALL LETTER H', 'LATIN SMALL LETTER I', 'LATIN SMALL LETTER J', 'LATIN SMALL LETTER K', 'LATIN SMALL LETTER L', 'LATIN SMALL LETTER M', 'LATIN SMALL LETTER N', 'LATIN SMALL LETTER O', 'LATIN SMALL LETTER P', 'LATIN SMALL LETTER Q', 'LATIN SMALL LETTER R', 'LATIN SMALL LETTER S', 'LATIN SMALL LETTER T', 'LATIN SMALL LETTER U', 'LATIN SMALL LETTER V', 'LATIN SMALL LETTER W', 'LATIN SMALL LETTER X', 'LATIN SMALL LETTER Y', 'LATIN SMALL LETTER Z', 'LEFT CURLY BRACKET', 'VERTICAL LINE', 'RIGHT CURLY BRACKET', 'TILDE', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', 'NO-BREAK SPACE', 'INVERTED EXCLAMATION MARK', 'CENT SIGN', 'POUND SIGN', 'CURRENCY SIGN', 'YEN SIGN', 'BROKEN BAR', 'SECTION SIGN', 'DIAERESIS', 'COPYRIGHT SIGN', 'FEMININE ORDINAL INDICATOR', 'LEFT-POINTING DOUBLE ANGLE QUOTATION MARK', 'NOT SIGN', 'SOFT HYPHEN', 'REGISTERED SIGN', 'MACRON', 'DEGREE SIGN', 'PLUS-MINUS SIGN', 'SUPERSCRIPT TWO', 'SUPERSCRIPT THREE', 'ACUTE ACCENT', 'MICRO SIGN', 'PILCROW SIGN', 'MIDDLE DOT', 'CEDILLA', 'SUPERSCRIPT ONE', 'MASCULINE ORDINAL INDICATOR', 'RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK', 'VULGAR FRACTION ONE QUARTER', 'VULGAR FRACTION ONE HALF', 'VULGAR FRACTION THREE QUARTERS', 'INVERTED QUESTION MARK', 'LATIN CAPITAL LETTER A WITH GRAVE', 'LATIN CAPITAL LETTER A WITH ACUTE', 'LATIN CAPITAL LETTER A WITH CIRCUMFLEX', 'LATIN CAPITAL LETTER A WITH TILDE', 'LATIN CAPITAL LETTER A WITH DIAERESIS', 'LATIN CAPITAL LETTER A WITH RING ABOVE', 'LATIN CAPITAL LETTER AE', 'LATIN CAPITAL LETTER C WITH CEDILLA', 'LATIN CAPITAL LETTER E WITH GRAVE', 'LATIN CAPITAL LETTER E WITH ACUTE', 'LATIN CAPITAL LETTER E WITH CIRCUMFLEX', 'LATIN CAPITAL LETTER E WITH DIAERESIS', 'LATIN CAPITAL LETTER I WITH GRAVE', 'LATIN CAPITAL LETTER I WITH ACUTE', 'LATIN CAPITAL LETTER I WITH CIRCUMFLEX', 'LATIN CAPITAL LETTER I WITH DIAERESIS', 'LATIN CAPITAL LETTER ETH', 'LATIN CAPITAL LETTER N WITH TILDE', 'LATIN CAPITAL LETTER O WITH GRAVE', 'LATIN CAPITAL LETTER O WITH ACUTE', 'LATIN CAPITAL LETTER O WITH CIRCUMFLEX', 'LATIN CAPITAL LETTER O WITH TILDE', 'LATIN CAPITAL LETTER O WITH DIAERESIS', 'MULTIPLICATION SIGN', 'LATIN CAPITAL LETTER O WITH STROKE', 'LATIN CAPITAL LETTER U WITH GRAVE', 'LATIN CAPITAL LETTER U WITH ACUTE', 'LATIN CAPITAL LETTER U WITH CIRCUMFLEX', 'LATIN CAPITAL LETTER U WITH DIAERESIS', 'LATIN CAPITAL LETTER Y WITH ACUTE', 'LATIN CAPITAL LETTER THORN', 'LATIN SMALL LETTER SHARP S', 'LATIN SMALL LETTER A WITH GRAVE', 'LATIN SMALL LETTER A WITH ACUTE', 'LATIN SMALL LETTER A WITH CIRCUMFLEX', 'LATIN SMALL LETTER A WITH TILDE', 'LATIN SMALL LETTER A WITH DIAERESIS', 'LATIN SMALL LETTER A WITH RING ABOVE', 'LATIN SMALL LETTER AE', 'LATIN SMALL LETTER C WITH CEDILLA', 'LATIN SMALL LETTER E WITH GRAVE', 'LATIN SMALL LETTER E WITH ACUTE', 'LATIN SMALL LETTER E WITH CIRCUMFLEX', 'LATIN SMALL LETTER E WITH DIAERESIS', 'LATIN SMALL LETTER I WITH GRAVE', 'LATIN SMALL LETTER I WITH ACUTE', 'LATIN SMALL LETTER I WITH CIRCUMFLEX', 'LATIN SMALL LETTER I WITH DIAERESIS', 'LATIN SMALL LETTER ETH', 'LATIN SMALL LETTER N WITH TILDE', 'LATIN SMALL LETTER O WITH GRAVE', 'LATIN SMALL LETTER O WITH ACUTE', 'LATIN SMALL LETTER O WITH CIRCUMFLEX', 'LATIN SMALL LETTER O WITH TILDE', 'LATIN SMALL LETTER O WITH DIAERESIS', 'DIVISION SIGN', 'LATIN SMALL LETTER O WITH STROKE', 'LATIN SMALL LETTER U WITH GRAVE', 'LATIN SMALL LETTER U WITH ACUTE', 'LATIN SMALL LETTER U WITH CIRCUMFLEX', 'LATIN SMALL LETTER U WITH DIAERESIS', 'LATIN SMALL LETTER Y WITH ACUTE', 'LATIN SMALL LETTER THORN', 'LATIN SMALL LETTER Y WITH DIAERESIS']

def name(char, default=None):
    if not isinstance(char, str) or len(char) != 1:
        raise TypeError('argument must be a unicode character')
    n = ord(char)
    if n < 256 and _names[n]:
        return _names[n]
    if default is not None:
        return default
    raise ValueError('no such name')

def lookup(name):
    return __sre_native(5, name.upper())
