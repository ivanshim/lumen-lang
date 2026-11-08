# Preserve native instruction-cache records alongside marshal's value streams.
import types as _code_types

_value_dumps = dumps
_value_loads = loads

# A path back to the root code needs the reference writer; repeated containers
# are allowed in the native record.
def _native_record(value):
    seen = set()
    def walk(item):
        if item is value:
            return False
        if isinstance(item, _code_types.CodeType):
            key = id(item)
            if key in seen:
                return True
            seen.add(key)
            return all(walk(const) for const in item.co_consts)
        if isinstance(item, (tuple, list, set, frozenset)):
            key = id(item)
            if key in seen:
                return True
            seen.add(key)
            return all(walk(part) for part in item)
        if isinstance(item, dict):
            key = id(item)
            if key in seen:
                return True
            seen.add(key)
            return all(walk(part) for part in item.values())
        return True
    return all(walk(const) for const in value.co_consts)

def dumps(value, version=version, /, *, allow_code=True):
    if isinstance(value, _code_types.CodeType):
        if not allow_code:
            raise ValueError('marshalling code objects is disallowed')
        if not (getattr(value, "\0replace_constants", False)
                or getattr(value, "\0literal_overrides", False)) and _native_record(value):
            try:
                return _code_prefix + _code_pack(value)
            except (TypeError, ValueError):
                # Function handles and constants outside the cache ABI use
                # the marshal reconstruction record rather than a cache graph.
                pass
    return _value_dumps(value, version, allow_code=allow_code)

def loads(data, /, *, allow_code=True):
    if not isinstance(data, (bytes, bytearray, str)):
        data = memoryview(data).tobytes()
    if isinstance(data, (bytes, bytearray)) and data.startswith(_code_prefix):
        if not allow_code:
            raise ValueError('unmarshalling code objects is disallowed')
        return _code_unpack(data[len(_code_prefix):])
    return _value_loads(data, allow_code=allow_code)
