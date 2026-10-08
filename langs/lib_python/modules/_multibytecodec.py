# Multibyte codec state and stream contracts: CPython v3.14.8 Modules/cjkcodecs.
import codecs
_mapping = __multibyte_native

class _CodecCall:
    def __init__(self, codec, encoding):
        self.codec = codec
        self.encoding = encoding
    def __call__(self, input, errors='strict'):
        if self.encoding:
            instance = MultibyteIncrementalEncoder(errors, self.codec)
            return instance.encode(input, True), len(input)
        instance = MultibyteIncrementalDecoder(errors, self.codec)
        return instance.decode(input, True), len(input)

class MultibyteCodec:
    def __init__(self, name):
        self.name = name
        self.encode = _CodecCall(self, True)
        self.decode = _CodecCall(self, False)

def _handler(name, error, length):
    value, position = codecs.lookup_error(name)(error)
    if position < 0:
        position += length
    if position < 0 or position > length:
        raise IndexError('position from error handler out of bounds')
    return value, position

class _State:
    def _initialize(self, decoder):
        self._state = [0] * 8
        if self.codec.name.startswith('iso2022_'):
            self._state[0] = 66
            self._state[1] = 66
            if decoder:
                self._state[2] = 66
        self._pending = b'' if decoder else ''

    def _packed(self):
        return int.from_bytes(bytes(self._state), 'little')

    def _designation(self):
        group = 1 if self.codec.name == 'iso2022_kr' and self._state[4] & 1 else 0
        mark = self._state[group]
        if mark == 66:
            return b''
        if group == 1:
            return b'\x1b$)C\x0e'
        if mark & 128:
            return (b'\x1b$' if mark in (192,193,194) else b'\x1b$(') + bytes([mark & 127])
        return b'\x1b(' + bytes([mark])

class MultibyteIncrementalEncoder(_State):
    def __init__(self, errors='strict', codec=None):
        if codec is not None:
            self.codec = codec
        if not isinstance(errors, str):
            raise TypeError('errors must be str')
        self.errors = errors
        self._initialize(False)

    def _emit(self, wire):
        name = self.codec.name
        if name == 'hz':
            if wire.startswith(b'~{'):
                prefix = b'' if self._state[0] else b'~{'
                self._state[0] = 1
                return prefix + wire[2:]
            prefix = b'~}' if self._state[0] else b''
            self._state[0] = 0
            return prefix + wire
        if not name.startswith('iso2022_'):
            return wire
        if name == 'iso2022_kr':
            if wire.startswith(b'\x1b$)C'):
                prefix = b''
                if self._state[1] != 195:
                    prefix += b'\x1b$)C'
                    self._state[1] = 195
                if not self._state[4] & 1:
                    prefix += b'\x0e'
                    self._state[4] |= 1
                return prefix + wire[5:]
            prefix = b'\x0f' if self._state[4] & 1 else b''
            self._state[4] &= ~1
            return prefix + wire
        if wire.startswith(b'\x1b.'):
            mark = wire[2]
            prefix = b'' if self._state[2] == mark else wire[:3]
            self._state[2] = mark
            return prefix + wire[3:]
        if wire.startswith(b'\x1b') and len(wire) >= 3:
            length = 4 if wire[1:3] == b'$(' else 3
            mark = wire[length-1] | (128 if wire[1] == 36 else 0)
            prefix = b'' if self._state[0] == mark else wire[:length]
            self._state[0] = mark
            return prefix + wire[length:]
        prefix = b'\x1b(B' if self._state[0] != 66 else b''
        self._state[0] = 66
        return prefix + wire

    def encode(self, input, final=False):
        if not isinstance(input, str):
            raise TypeError('encoding with ' + self.codec.name + ' requires str')
        text = self._pending + input
        self._pending = ''
        result = b''
        pos = 0
        while pos < len(text):
            first = ord(text[pos])
            if pos+1 == len(text) and not final and _mapping(3, self.codec.name, first, 0):
                self._pending = text[pos:]
                break
            wire = _mapping(0, self.codec.name, first, ord(text[pos+1])) if pos+1 < len(text) else None
            consumed = 2
            if wire is None:
                wire = _mapping(0, self.codec.name, first, 0)
                consumed = 1
            if wire is None:
                error = UnicodeEncodeError(self.codec.name, text, pos, pos+1, 'illegal multibyte sequence')
                replacement, pos = _handler(self.errors, error, len(text))
                result += replacement if isinstance(replacement, bytes) else self.encode(replacement, False)
            else:
                result += self._emit(wire)
                pos += consumed
        if final:
            if self.codec.name == 'hz' and self._state[0]:
                result += b'~}'
                self._state[0] = 0
            elif self.codec.name.startswith('iso2022_'):
                if self._state[4] & 1:
                    result += b'\x0f'
                    self._state[4] &= ~1
                if self._state[0] != 66:
                    result += b'\x1b(B'
                    self._state[0] = 66
        return result

    def getstate(self):
        pending = self._pending.encode('utf-8')
        return int.from_bytes(bytes([len(pending)]) + pending + bytes(self._state), 'little')

    def setstate(self, state):
        if not isinstance(state, int):
            raise TypeError('state must be an integer')
        raw = state.to_bytes(17, 'little')
        count = raw[0]
        if count > 8:
            raise UnicodeError('pending buffer too large')
        self._pending = raw[1:1+count].decode('utf-8')
        self._state = list(raw[1+count:9+count])

    def reset(self):
        self._initialize(False)

class MultibyteIncrementalDecoder(_State):
    def __init__(self, errors='strict', codec=None):
        if codec is not None:
            self.codec = codec
        if not isinstance(errors, str):
            raise TypeError('errors must be str')
        self.errors = errors
        self._initialize(True)

    def decode(self, input, final=False):
        if isinstance(input, str) or isinstance(input, int):
            raise TypeError('a bytes-like object is required, not ' + type(input).__name__)
        data = self._pending + bytes(input)
        self._pending = b''
        pos = 0
        result = ''
        name = self.codec.name
        while pos < len(data):
            begin = pos
            prefix = b''
            width = 1
            if name == 'hz':
                if data[pos] == 126:
                    if pos+1 == len(data):
                        if not final:
                            break
                    else:
                        following = data[pos+1]
                        if following in (123,125):
                            self._state[0] = int(following == 123)
                            pos += 2
                            continue
                        if following == 10:
                            pos += 2
                            continue
                        if following == 126:
                            result += '~'
                            pos += 2
                            continue
                if self._state[0]:
                    prefix = b'~{'
                    width = 2
            elif name.startswith('iso2022_'):
                byte = data[pos]
                if byte == 27:
                    end = pos+1
                    while end < len(data) and not (64 <= data[end] <= 90):
                        end += 1
                    if end == len(data):
                        if not final:
                            break
                    else:
                        escape = data[pos:end+1]
                        mark = data[end]
                        if escape[:2] in (b'\x1b(', b'\x1b$'):
                            group = 1 if escape.startswith(b'\x1b$)') else 0
                            self._state[group] = mark | (128 if escape[1] == 36 else 0)
                            pos = end+1
                            continue
                        if escape.startswith(b'\x1b.'):
                            self._state[2] = mark
                            pos = end+1
                            continue
                        if escape == b'\x1bN':
                            prefix = b'\x1b.' + bytes([self._state[2]]) + b'\x1bN'
                            pos = end+1
                            width = 1
                elif name == 'iso2022_kr' and byte in (14,15):
                    if byte == 14:
                        self._state[4] |= 1
                    else:
                        self._state[4] &= ~1
                    pos += 1
                    continue
                if not prefix:
                    prefix = self._designation()
                    group = 1 if name == 'iso2022_kr' and self._state[4] & 1 else 0
                    width = 2 if self._state[group] & 128 else 1
            else:
                # Search an atomic unit, keeping a valid unfinished prefix buffered.
                width = 1
                while width <= 8 and pos+width <= len(data):
                    unit = data[pos:pos+width]
                    if _mapping(1, name, unit, 0) is not None:
                        break
                    if not _mapping(2, name, unit, 0):
                        break
                    width += 1
                if width == 1 and data[pos] >= 128 and _mapping(1, name, data[pos:pos+1], 0) is None and name not in ('cp932', 'shift_jis', 'shift_jis_2004', 'shift_jisx0213'):
                    width = 3 if name.startswith('euc_j') and data[pos] == 143 else 2
            if pos+width > len(data):
                if not final:
                    pos = begin
                    break
                decoded = None
                reason = 'incomplete multibyte sequence'
                end = len(data)
            else:
                unit = prefix + data[pos:pos+width]
                decoded = _mapping(1, name, unit, 0)
                reason = 'illegal multibyte sequence'
                end = begin+1
            if decoded is None:
                error = UnicodeDecodeError(name, data, begin, end, reason)
                replacement, pos = _handler(self.errors, error, len(data))
                if not isinstance(replacement, str):
                    raise TypeError('decoding error handler must return (str, int) tuple')
                result += replacement
            else:
                result += decoded
                pos += width
        self._pending = data[pos:]
        return result

    def getstate(self):
        return self._pending, self._packed()

    def setstate(self, state):
        if not isinstance(state, tuple) or len(state) != 2 or not isinstance(state[0], bytes) or not isinstance(state[1], int):
            raise TypeError('setstate(): illegal state argument')
        self._pending = state[0]
        self._state = list(state[1].to_bytes(8, 'little'))

    def reset(self):
        self._initialize(True)

class MultibyteStreamReader:
    def __init__(self, stream, errors='strict'):
        codecs.StreamReader.__init__(self, stream, errors)
        self._decoder = MultibyteIncrementalDecoder(errors, self.codec)
        self.decode = self._decode

    def _decode(self, data, errors='strict'):
        self._decoder.errors = errors
        return self._decoder.decode(data, False), len(data)

    def reset(self):
        codecs.StreamReader.reset(self)
        self._decoder.reset()

class MultibyteStreamWriter:
    def __init__(self, stream, errors='strict'):
        codecs.StreamWriter.__init__(self, stream, errors)
        self._encoder = MultibyteIncrementalEncoder(errors, self.codec)
        self.encode = self._encode

    def _encode(self, text, errors='strict'):
        self._encoder.errors = errors
        return self._encoder.encode(text, False), len(text)

    def reset(self):
        if not self._encoder._pending:
            return
        ending = self._encoder.encode('', True)
        if ending:
            self.stream.write(ending)
        self._encoder.reset()
