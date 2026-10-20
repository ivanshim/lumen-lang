# Native compiled-code records use the actual kernel instruction format and ABI.
import types as _code_types

_value_dumps = dumps
_value_loads = loads
_value_prefix = b"\x00LumenValue\x01"

_code_prefix = b'\x00LumenCode\x01'

class _CodeWriter(_Writer):
    def __init__(self, allow_code):
        super().__init__(False, _unwritable)
        self.allow_code = allow_code

    def put(self, value):
        if not isinstance(value, _code_types.CodeType):
            return super().put(value)
        if not self.allow_code:
            raise ValueError('marshalling code objects is disallowed')
        if self.held(value):
            return
        record = _code_pack(value).hex()
        self.pieces.append('C' + str(len(record)) + ';' + record)


class _CodeReader(_Reader):
    def __init__(self, text, allow_code):
        super().__init__(text)
        self.allow_code = allow_code

    def get(self):
        if self.at >= len(self.text) or self.text[self.at] != 'C':
            return super().get()
        if not self.allow_code:
            raise ValueError('unmarshalling code objects is disallowed')
        self.at += 1
        where = self.reserve()
        value = _code_unpack(bytes.fromhex(self.characters(self.count())))
        self.memo[where] = value
        return value


def dumps(value, version=version, /, *, allow_code=True):
    if isinstance(value, _code_types.CodeType):
        if not allow_code:
            raise ValueError('marshalling code objects is disallowed')
        return _code_prefix + _code_pack(value)
    try:
        return _value_dumps(value, version, allow_code=allow_code)
    except ValueError:
        pass
    writer = _CodeWriter(allow_code)
    writer.put(value)
    return _value_prefix + writer.text().encode()

def loads(data, /, *, allow_code=True):
    data = memoryview(data).tobytes()
    if data.startswith(_code_prefix):
        if not allow_code:
            raise ValueError('unmarshalling code objects is disallowed')
        return _code_unpack(data[len(_code_prefix):])
    if not data:
        raise EOFError('EOF read where object expected')
    if data.startswith(_value_prefix):
        return _CodeReader(data[len(_value_prefix):].decode(), allow_code).get()
    return _value_loads(data, allow_code=allow_code)

def dump(value, file, version=version, /, *, allow_code=True):
    file.write(dumps(value, version, allow_code=allow_code))

def load(file, /, *, allow_code=True):
    return loads(file.read(), allow_code=allow_code)
