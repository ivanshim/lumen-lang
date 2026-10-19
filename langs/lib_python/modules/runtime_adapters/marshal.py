# Native compiled-code records use the actual kernel instruction format and ABI.
import types as _code_types

_value_dumps = dumps
_value_loads = loads
_code_prefix = b'\x00LumenCode\x01'

def dumps(value, version=version, /, *, allow_code=True):
    if isinstance(value, _code_types.CodeType):
        if not allow_code:
            raise ValueError('marshalling code objects is disallowed')
        return _code_prefix + _code_pack(value)
    return _value_dumps(value, version)

def loads(data, /, *, allow_code=True):
    data = memoryview(data).tobytes()
    if data.startswith(_code_prefix):
        if not allow_code:
            raise ValueError('unmarshalling code objects is disallowed')
        return _code_unpack(data[len(_code_prefix):])
    return _value_loads(data)

def dump(value, file, version=version, /, *, allow_code=True):
    file.write(dumps(value, version, allow_code=allow_code))

def load(file, /, *, allow_code=True):
    return loads(file.read(), allow_code=allow_code)
