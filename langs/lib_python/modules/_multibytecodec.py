# The machinery of the reference's multibyte codecs, written in Python: a
# codec object that encodes and decodes whole strings, and the incremental
# and stream classes the encodings package builds on it. A codec supplies
# the character tables; this module runs them and answers errors with
# the handlers codecs registers.

import codecs


class _Illegal(Exception):
    pass


def _handler(errors):
    if errors is None:
        errors = 'strict'
    if not isinstance(errors, str):
        raise TypeError('errors must be a string')
    return codecs.lookup_error(errors), errors


def _encode_run(spec, text, errors, pending, final):
    # Encodes pending + text. A character that may begin a pair is held
    # back at the end of the text unless this is the last of it.
    handler, errors = _handler(errors)
    queue = pending + text
    out = bytearray()
    i = 0
    size = len(queue)
    while i < size:
        c = queue[i]
        if c in spec.pair_first:
            if i + 1 < size:
                code = spec.pairs.get((c, queue[i + 1]))
                if code is not None:
                    out += code
                    i += 2
                    continue
            elif not final:
                return bytes(out), c
        code = spec.encode_char(c)
        if code is not None:
            out += code
            i += 1
            continue
        end = i + 1
        exc = UnicodeEncodeError(spec.name, queue, i, end, 'illegal multibyte sequence')
        replacement, resume = handler(exc)
        if isinstance(replacement, str):
            replaced = bytearray()
            for letter in replacement:
                code = spec.encode_char(letter)
                if code is None:
                    raise UnicodeEncodeError(spec.name, queue, i, end, 'illegal multibyte sequence')
                replaced += code
            out += replaced
        elif isinstance(replacement, bytes):
            out += replacement
        else:
            raise TypeError("encoding error handler must return (str/bytes, int) tuple")
        if resume < 0:
            resume += size
        if not 0 <= resume <= size:
            raise IndexError('position %d from error handler out of bounds' % resume)
        i = resume
    return bytes(out), ''


def _decode_run(spec, data, errors, final):
    # Decodes data as far as it is whole; returns the text and how many
    # bytes were used. An unfinished sequence at the end is an error only
    # when this is the last of the data.
    handler, errors = _handler(errors)
    data = bytes(data)
    out = []
    i = 0
    size = len(data)
    while i < size:
        first = data[i]
        if first < 0x80:
            out.append(chr(first))
            i += 1
            continue
        try:
            text, used = spec.decode_at(data, i)
        except _Illegal as bad:
            length, reason = bad.args
        else:
            if text is not None:
                out.append(text)
                i += used
                continue
            if not final:
                break
            length, reason = size - i, 'incomplete multibyte sequence'
        exc = UnicodeDecodeError(spec.name, data, i, i + length, reason)
        replacement, resume = handler(exc)
        if not isinstance(replacement, str):
            raise TypeError("decoding error handler must return (str, int) tuple")
        out.append(replacement)
        if resume < 0:
            resume += size
        if not 0 <= resume <= size:
            raise IndexError('position %d from error handler out of bounds' % resume)
        i = resume
    return ''.join(out), i


class MultibyteCodec:
    def __init__(self, spec):
        self._spec = spec
        self.name = spec.name

    def encode(self, input, errors=None):
        if not isinstance(input, str):
            raise TypeError("argument 1 must be str, not %s" % type(input).__name__)
        data, _ = _encode_run(self._spec, input, errors, '', True)
        return data, len(input)

    def decode(self, input, errors=None):
        data = bytes(memoryview(input)) if not isinstance(input, bytes) else input
        text, _ = _decode_run(self._spec, data, errors, True)
        return text, len(data)


class MultibyteIncrementalEncoder:
    def __init__(self, errors='strict'):
        self.errors = errors
        self._pending = ''

    def encode(self, input, final=False):
        data, self._pending = _encode_run(self.codec._spec, input, self.errors, self._pending, final)
        return data

    def reset(self):
        self._pending = ''

    def getstate(self):
        return int.from_bytes(self._pending.encode('utf-32-le'), 'little') if self._pending else 0

    def setstate(self, state):
        if not isinstance(state, int):
            raise TypeError('an integer is required')
        if state < 0 or state.bit_length() > 32:
            raise UnicodeError('pending buffer overflow')
        self._pending = state.to_bytes(4, 'little').decode('utf-32-le') if state else ''


class MultibyteIncrementalDecoder:
    def __init__(self, errors='strict'):
        self.errors = errors
        self._pending = b''

    def decode(self, input, final=False):
        data = self._pending + bytes(memoryview(input))
        text, used = _decode_run(self.codec._spec, data, self.errors, final)
        self._pending = data[used:]
        return text

    def reset(self):
        self._pending = b''

    def getstate(self):
        return self._pending, 0

    def setstate(self, state):
        buffer, flag = state
        if not isinstance(buffer, bytes):
            raise TypeError('state[0] must be bytes')
        self._pending = buffer


class MultibyteStreamReader:
    def __init__(self, stream, errors='strict'):
        self.stream = stream
        self.errors = errors
        self._pending = b''

    def _iread(self, method, size):
        if size is not None and size < 0:
            size = None
        while True:
            call = getattr(self.stream, method)
            data = call() if size is None else call(size)
            final = size is None or not data
            data = self._pending + data
            text, used = _decode_run(self.codec._spec, data, self.errors, final)
            self._pending = data[used:]
            if text or final:
                return text

    def read(self, size=None):
        return self._iread('read', size)

    def readline(self, size=None):
        return self._iread('readline', size)

    def readlines(self, sizehint=None):
        return self._iread('read', sizehint).splitlines(True)

    def reset(self):
        self._pending = b''


class MultibyteStreamWriter:
    def __init__(self, stream, errors='strict'):
        self.stream = stream
        self.errors = errors
        self._pending = ''

    def write(self, strobj):
        data, self._pending = _encode_run(self.codec._spec, strobj, self.errors, self._pending, False)
        if data:
            self.stream.write(data)

    def writelines(self, lines):
        for line in lines:
            self.write(line)

    def reset(self):
        data, self._pending = _encode_run(self.codec._spec, '', self.errors, self._pending, True)
        if data:
            self.stream.write(data)


def __create_codec(spec):
    return MultibyteCodec(spec)
