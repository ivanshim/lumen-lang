# Native XZ codec interface used by the unchanged release lzma module.
# shortcut: streams buffer complete input/output; use incremental liblzma for large archives.
from operator import index

FORMAT_AUTO, FORMAT_XZ, FORMAT_ALONE, FORMAT_RAW = 0, 1, 2, 3
CHECK_NONE, CHECK_CRC32, CHECK_CRC64, CHECK_SHA256 = 0, 1, 4, 10
CHECK_ID_MAX, CHECK_UNKNOWN = 15, 16
PRESET_DEFAULT, PRESET_EXTREME = 6, 1 << 31
MODE_FAST, MODE_NORMAL = 1, 2
MF_HC3, MF_HC4, MF_BT2, MF_BT3, MF_BT4 = 3, 4, 18, 19, 20
FILTER_LZMA1, FILTER_LZMA2, FILTER_DELTA = 0x4000000000000001, 0x21, 3
FILTER_X86, FILTER_POWERPC, FILTER_IA64, FILTER_ARM, FILTER_ARMTHUMB, FILTER_SPARC = 4, 5, 6, 7, 8, 9

class LZMAError(Exception):
    pass

# Report the checks supported by the linked liblzma implementation.
def is_check_supported(check):
    return index(check) in (CHECK_NONE, CHECK_CRC32, CHECK_CRC64, CHECK_SHA256)

# Validate the LZMA1 properties used by raw ZIP streams.
def _lzma1_options(filter):
    if filter.get('id') != FILTER_LZMA1:
        raise NotImplementedError('only raw LZMA1 filter chains are supported')
    for key in filter:
        if key not in ('id', 'preset', 'dict_size', 'lc', 'lp', 'pb'):
            raise ValueError('unsupported LZMA1 option: ' + str(key))
    preset = index(filter.get('preset', PRESET_DEFAULT))
    level = preset & ~PRESET_EXTREME
    if not 0 <= level <= 9:
        raise LZMAError('invalid compression preset')
    sizes = (1 << 18, 1 << 20, 1 << 21, 1 << 22, 1 << 22, 1 << 23, 1 << 23, 1 << 24, 1 << 25, 1 << 26)
    dictionary = index(filter.get('dict_size', sizes[level]))
    lc, lp, pb = index(filter.get('lc', 3)), index(filter.get('lp', 0)), index(filter.get('pb', 2))
    if not 4096 <= dictionary <= 0xffffffff or not 0 <= lc <= 4 or not 0 <= lp <= 4 or lc + lp > 4 or not 0 <= pb <= 4:
        raise LZMAError('invalid LZMA1 properties')
    return preset, (dictionary, lc, lp, pb)

def _encode_filter_properties(filter):
    preset, options = _lzma1_options(filter)
    status, encoded = __lzma_encode__(b'', preset, CHECK_NONE, options)
    if status:
        raise LZMAError('invalid LZMA1 properties')
    return encoded[:5]

def _decode_filter_properties(filter_id, encoded):
    if index(filter_id) != FILTER_LZMA1:
        raise NotImplementedError('only raw LZMA1 filter chains are supported')
    encoded = bytes(encoded)
    if len(encoded) != 5 or encoded[0] >= 225:
        raise LZMAError('invalid LZMA1 properties')
    prop = encoded[0]
    lc, rest = prop % 9, prop // 9
    lp, pb = rest % 5, rest // 5
    dictionary = int.from_bytes(encoded[1:], 'little')
    return {'id': FILTER_LZMA1, 'dict_size': max(dictionary, 4096), 'lc': lc, 'lp': lp, 'pb': pb}

def _raw_filter(filters):
    if filters is None:
        raise ValueError('must specify filters for FORMAT_RAW')
    if len(filters) != 1:
        raise NotImplementedError('only raw LZMA1 filter chains are supported')
    return filters[0]

class LZMACompressor:
    # Buffer input for the platform's complete-stream encoder.
    def __init__(self, format=FORMAT_XZ, check=-1, preset=None, filters=None):
        format = index(format)
        self._raw = format == FORMAT_RAW
        self._properties = None
        if self._raw:
            if preset is not None or check not in (-1, CHECK_NONE):
                raise ValueError('invalid raw compression options')
            preset, self._properties = _lzma1_options(_raw_filter(filters))
        elif format != FORMAT_XZ:
            raise NotImplementedError('only XZ and raw LZMA1 encoding are supported')
        elif filters is not None:
            raise NotImplementedError('XZ filter chains are not supported')
        self._preset = PRESET_DEFAULT if preset is None else index(preset)
        self._check = CHECK_CRC64 if check == -1 else index(check)
        status, unused = self._encode(b'')
        if status:
            raise LZMAError('invalid compression options')
        self._input = bytearray()
        self._finished = False

    def _encode(self, data):
        if self._raw:
            return __lzma_encode__(data, self._preset, CHECK_NONE, self._properties)
        return __lzma_encode__(data, self._preset, self._check)

    # Retain a bytes-like input until the final compressed stream is requested.
    def compress(self, data):
        if self._finished:
            raise ValueError('Compressor has been flushed')
        if not isinstance(data, (bytes, bytearray, memoryview)):
            raise TypeError('a bytes-like object is required')
        self._input.extend(bytes(data))
        return b''

    # Return one real XZ stream and prevent subsequent use of the compressor.
    def flush(self):
        if self._finished:
            raise ValueError('Compressor has been flushed')
        status, output = self._encode(bytes(self._input))
        if status:
            raise LZMAError('compression failed')
        self._input.clear()
        self._finished = True
        return output[13:] if self._raw else output

class LZMADecompressor:
    # Retain partial input and bounded output around the complete-stream decoder.
    def __init__(self, format=FORMAT_AUTO, memlimit=None, filters=None):
        self._format = index(format)
        self._raw_header = b''
        if self._format == FORMAT_RAW:
            if memlimit is not None:
                raise ValueError('cannot specify memlimit for FORMAT_RAW')
            self._raw_header = _encode_filter_properties(_raw_filter(filters)) + b'\xff' * 8
            self._format = FORMAT_ALONE
            filters = None
        if self._format not in (FORMAT_AUTO, FORMAT_XZ, FORMAT_ALONE):
            raise ValueError('invalid container format')
        if filters is not None:
            raise NotImplementedError('raw LZMA filter chains are not supported')
        self._limit = 18446744073709551615 if memlimit is None else index(memlimit)
        if not 0 <= self._limit <= 18446744073709551615:
            raise OverflowError('invalid memory limit')
        self._input = bytearray()
        self._output = b''
        self._position = 0
        self._complete = False
        self.eof = False
        self.needs_input = True
        self.unused_data = b''
        self.check = CHECK_UNKNOWN

    # Decode complete input and keep any bounded output for the next read.
    def decompress(self, data, max_length=-1):
        if self.eof:
            raise EOFError('Already at end of stream')
        if not isinstance(data, (bytes, bytearray, memoryview)):
            raise TypeError('a bytes-like object is required')
        max_length = index(max_length)
        self._input.extend(bytes(data))
        if not self._complete:
            status, output, consumed = __lzma_decode__(self._raw_header + bytes(self._input), self._format, str(self._limit))
            if status == 10:
                self.needs_input = True
                return b''
            if status:
                raise LZMAError('invalid or incomplete compressed data')
            self._output = output
            self.unused_data = bytes(self._input[consumed - len(self._raw_header):])
            self._input.clear()
            self._complete = True
        end = len(self._output) if max_length < 0 else min(len(self._output), self._position + max_length)
        output = self._output[self._position:end]
        self._position = end
        self.eof = self._complete and self._position == len(self._output)
        if self.eof:
            self._output = b''
            self._position = 0
        self.needs_input = not self._complete
        return output
