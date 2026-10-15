# Shift-JIS uses the existing multibyte mapping embedded from mbstring.php.
# The hex rows are shared data; no PHP code is evaluated here.
import codecs
import _codec_runtime as _handlers
from builtins import _buffer_view

_rows = None
_backward = None

def _tables():
    global _rows, _backward
    if _rows is None:
        _rows = {}
        _backward = {}
        for line in _mapping_source.splitlines():
            line = line.strip()
            if line.startswith('0x') and ' => "' in line:
                key, row = line.split(' => "', 1)
                lead = int(key, 16)
                row = row.split('"', 1)[0]
                _rows[lead] = row
        _backward[0xa5] = b'\x5c'
        _backward[0x203e] = b'\x7e'
    return _rows, _backward

def _error_mode(errors, operation):
    if errors is None:
        return 'strict'
    if not isinstance(errors, str):
        raise TypeError(operation + "() argument 'errors' must be str or None, not " + type(errors).__name__)
    return errors

def _read_buffer(data):
    try:
        with _buffer_view(data, 0) as view:
            if not view.c_contiguous:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            return view.tobytes()
    except TypeError as error:
        message = str(error)
        if message.startswith('memoryview: a bytes-like object is required'):
            raise TypeError(message[len('memoryview: '):]) from None
        raise

def _incremental_errors(errors, kind):
    if not isinstance(errors, str):
        name = 'None' if errors is None else type(errors).__name__
        raise TypeError(kind + '() argument 1 must be str, not ' + name)
    return errors

def encode(text, errors='strict'):
    errors = _error_mode(errors, 'encode')
    if not isinstance(text, str):
        text = str(text)
    original = text
    text = str.__str__(text)
    rows, backward = _tables()
    output = []
    position = 0
    while position < len(text):
        number = ord(text[position])
        if number < 128:
            encoded = bytes([number])
        elif 0xff61 <= number <= 0xff9f:
            encoded = bytes([number - 0xff61 + 0xa1])
        else:
            encoded = backward.get(number)
            if encoded is None and number <= 0xffff:
                marker = '%04x' % number
                for lead, row in rows.items():
                    offset = row.find(marker)
                    while offset >= 0 and offset % 4:
                        offset = row.find(marker, offset + 1)
                    if offset >= 0:
                        place = offset // 4
                        encoded = bytes([lead, place + (0x40 if place < 63 else 0x41)])
                        backward[number] = encoded
                        break
        if encoded is None:
            replacement, position = _handlers._encode_error('shift_jis', original, position, position + 1, 'illegal multibyte sequence', errors)
            encoded = replacement if isinstance(replacement, bytes) else encode(replacement, 'strict')[0]
        else:
            position += 1
        output.extend(encoded)
    return bytes(output), len(text)

def _decode(data, errors, final):
    rows, backward = _tables()
    data = _read_buffer(data)
    errors = _error_mode(errors, 'decode')
    result = []
    position = 0
    while position < len(data):
        lead = data[position]
        if lead < 128:
            result.append(chr(lead))
            position += 1
            continue
        if 0xa1 <= lead <= 0xdf:
            result.append(chr(lead - 0xa1 + 0xff61))
            position += 1
            continue
        number = 0
        reason = 'illegal multibyte sequence'
        if 0x81 <= lead <= 0x9f or 0xe0 <= lead <= 0xef:
            if position + 1 == len(data):
                if not final:
                    break
                reason = 'incomplete multibyte sequence'
            else:
                trail = data[position + 1]
                if 0x40 <= trail <= 0xfc and trail != 0x7f:
                    place = trail - (0x40 if trail < 0x7f else 0x41)
                    row = rows.get(lead, '')
                    if place * 4 + 4 <= len(row):
                        number = int(row[place * 4:place * 4 + 4], 16)
        if number:
            result.append(chr(number))
            position += 2
        else:
            replacement, position = _handlers._decode_error('shift_jis', data, position, position + 1, reason, errors)
            result.append(replacement)
    return ''.join(result), position

def decode(data, errors='strict'):
    return _decode(data, errors, True)

class IncrementalEncoder(codecs.IncrementalEncoder):
    def __init__(self, errors='strict'):
        super().__init__(_incremental_errors(errors, 'IncrementalEncoder'))
    def encode(self, input, final=False):
        return encode(input, self.errors)[0]

class IncrementalDecoder(codecs.IncrementalDecoder):
    def __init__(self, errors='strict'):
        super().__init__(_incremental_errors(errors, 'IncrementalDecoder'))
        self.buffer = b''
    def decode(self, input, final=False):
        data = self.buffer + _read_buffer(input)
        text, used = _decode(data, self.errors, final)
        self.buffer = data[used:]
        return text
    def reset(self):
        self.buffer = b''
    def getstate(self):
        return self.buffer, 0
    def setstate(self, state):
        self.buffer = state[0]

class StreamReader(codecs.StreamReader):
    def decode(self, input, errors='strict'):
        return _decode(input, errors, False)

class StreamWriter(codecs.StreamWriter):
    encode = staticmethod(encode)

def getregentry():
    return codecs.CodecInfo(name='shift_jis', encode=encode, decode=decode,
        incrementalencoder=IncrementalEncoder, incrementaldecoder=IncrementalDecoder,
        streamreader=StreamReader, streamwriter=StreamWriter)
