# The Japanese multibyte codecs the library supplies, EUC-JP and
# EUC-JIS-2004: one-, two- and three-byte sequences read from tables
# written out from the reference's codecs.

import _multibytecodec as _mb
from _codecs_jp_tables import (
    EUC_JP_PLANE1, EUC_JP_PLANE2, EUC_JP_EXTRAS,
    JIS2004_PLANE1, JIS2004_PLANE2, JIS2004_PAIRS, JIS2004_EXTRAS)


class _Spec:
    def __init__(self, name, plane1, plane2, pairs, extras):
        self.name = name
        self._plane1 = plane1
        self._plane2 = plane2
        self._pairs_by_code = pairs
        self._extras = extras
        # Whether any lead byte left alone at the end is taken for an
        # unfinished sequence, as the EUC-JP reader takes it.
        self._loose = True
        self._encode_map = None
        self.pairs = {}
        self.pair_first = set()
        for code, text in pairs.items():
            self.pairs[(text[0], text[1])] = bytes([code >> 8, code & 255])
            self.pair_first.add(text[0])

    def _map(self):
        found = {}
        for plane, prefix in ((self._plane1, b''), (self._plane2, b'\x8f')):
            for index, letter in enumerate(plane):
                if letter != '\0':
                    found.setdefault(letter, prefix + bytes([0xA1 + index // 94, 0xA1 + index % 94]))
        for byte in range(0xA1, 0xE0):
            found.setdefault(chr(0xFF61 + byte - 0xA1), bytes([0x8E, byte]))
        for letter, code in self._extras.items():
            found[letter] = code
        self._encode_map = found
        return found

    def encode_char(self, letter):
        if letter < '\x80':
            return letter.encode('ascii')
        found = self._encode_map
        if found is None:
            found = self._map()
        return found.get(letter)

    def decode_at(self, data, at):
        # (text, used) for the sequence at `at`, (None, 0) when it is
        # unfinished, or _Illegal(length, reason) when it is no sequence.
        first = data[at]
        size = len(data)
        if first == 0x8E:
            if at + 1 >= size:
                return None, 0
            second = data[at + 1]
            if 0xA1 <= second <= 0xDF:
                return chr(0xFF61 + second - 0xA1), 2
            raise _mb._Illegal(1, 'illegal multibyte sequence')
        if first == 0x8F:
            if at + 1 >= size:
                return None, 0
            second = data[at + 1]
            if not 0xA1 <= second <= 0xFE:
                raise _mb._Illegal(1, 'illegal multibyte sequence')
            if at + 2 >= size:
                return None, 0
            third = data[at + 2]
            if not 0xA1 <= third <= 0xFE:
                raise _mb._Illegal(1, 'illegal multibyte sequence')
            letter = self._plane2[(second - 0xA1) * 94 + third - 0xA1]
            if letter == '\0':
                raise _mb._Illegal(3, 'illegal multibyte sequence')
            return letter, 3
        if 0xA1 <= first <= 0xFE:
            if at + 1 >= size:
                return None, 0
            second = data[at + 1]
            if not 0xA1 <= second <= 0xFE:
                raise _mb._Illegal(1, 'illegal multibyte sequence')
            letter = self._plane1[(first - 0xA1) * 94 + second - 0xA1]
            if letter == '\0':
                pair = self._pairs_by_code.get((first << 8) | second)
                if pair is None:
                    raise _mb._Illegal(2, 'illegal multibyte sequence')
                return pair, 2
            return letter, 2
        if self._loose and at + 1 >= size:
            return None, 0
        raise _mb._Illegal(1, 'illegal multibyte sequence')


_codecs = {}


def getcodec(name):
    codec = _codecs.get(name)
    if codec is None:
        if name == 'euc_jp':
            spec = _Spec(name, EUC_JP_PLANE1, EUC_JP_PLANE2, {}, EUC_JP_EXTRAS)
        elif name == 'euc_jis_2004':
            spec = _Spec(name, JIS2004_PLANE1, JIS2004_PLANE2, JIS2004_PAIRS, JIS2004_EXTRAS)
        else:
            raise LookupError('no such codec is supported.')
        codec = _codecs[name] = _mb.__create_codec(spec)
    return codec
