# One-shot XZ and LZMA decoding through the platform codec.
from operator import index

_decode = __lzma_decode__
FORMAT_AUTO = 0
FORMAT_XZ = 1
FORMAT_ALONE = 2
FORMAT_RAW = 3

class LZMAError(Exception):
    pass

# Decode concatenated compressed streams, preserving decoder errors.
def decompress(data, format=FORMAT_AUTO, memlimit=None, filters=None):
    format = index(format)
    if filters is not None and format != FORMAT_RAW:
        raise ValueError('Cannot specify filters except with FORMAT_RAW')
    if format == FORMAT_RAW:
        # ponytail: raw filters require a filter-chain API; add it with raw decoding.
        raise NotImplementedError('raw LZMA filter chains are not supported')
    if format not in (FORMAT_AUTO, FORMAT_XZ, FORMAT_ALONE):
        raise ValueError('Invalid container format: ' + str(format))
    limit = 18446744073709551615 if memlimit is None else index(memlimit)
    if not 0 <= limit <= 18446744073709551615:
        raise OverflowError("can't convert negative int to unsigned" if limit < 0 else 'int too big to convert')
    if not isinstance(data, (bytes, bytearray, memoryview)):
        raise TypeError("a bytes-like object is required, not '" + type(data).__name__ + "'")
    data = bytes(data)
    results = []
    while True:
        code, output, consumed = _decode(data, format, str(limit))
        if code:
            if code == 5:
                raise MemoryError()
            if results and code in (7, 9):
                break
            messages = {6: 'Memory usage limit exceeded', 7: 'Input format not supported by decoder', 8: 'Invalid or unsupported options', 9: 'Corrupt input data', 10: 'Compressed data ended before the end-of-stream marker was reached', 11: 'Internal error'}
            raise LZMAError(messages.get(code, 'Unknown error'))
        results.append(output)
        data = data[consumed:]
        if not data:
            break
    return b''.join(results)
