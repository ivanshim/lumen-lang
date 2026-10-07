# Runtime binding for the native CPython v3.14.8 PickleBuffer interface.
# The pure Python accelerator fallback remains unchanged.
from _picklebuffer import PickleBuffer
_HAVE_PICKLE_BUFFER = True
__all__.append('PickleBuffer')

def save_picklebuffer(self, obj):
    if self.proto < 5:
        raise PicklingError("PickleBuffer can only be pickled with "
                            "protocol >= 5")
    with obj.raw() as m:
        if not m.contiguous:
            raise PicklingError("PickleBuffer can not be pickled when "
                                "pointing to a non-contiguous buffer")
        in_band = True
        if self._buffer_callback is not None:
            in_band = bool(self._buffer_callback(obj))
        if in_band:
            # Write data in-band
            # XXX The C implementation avoids a copy here
            buf = m.tobytes()
            if m.readonly:
                self._save_bytes_no_memo(buf)
            else:
                self._save_bytearray_no_memo(buf)
            self.memoize(obj)
        else:
            # Write data out-of-band
            self.write(NEXT_BUFFER)
            if m.readonly:
                self.write(READONLY_BUFFER)

save_picklebuffer.__qualname__ = '_Pickler.save_picklebuffer'
_Pickler.save_picklebuffer = save_picklebuffer
_Pickler.dispatch[PickleBuffer] = save_picklebuffer
del save_picklebuffer

# Decode protocol-0 byte escapes with the shared native codec, warning only
# after successful decoding and attributing it outside the pickle frames.
def _escape_decode(data):
    decoded, consumed, warning = __escape_decode_native__(data, None)
    if warning is not None:
        import warnings
        warnings.warn(warning, DeprecationWarning,
                      skip_file_prefixes=(__file__,))
    return decoded

# Preserve the canonical STRING parser and bind its byte codec to caller warnings.
def load_string(self):
    data = self.readline()[:-1]
    if len(data) >= 2 and data[0] == data[-1] and data[0] in b'"\'':
        data = data[1:-1]
    else:
        raise UnpicklingError("the STRING opcode argument must be quoted")
    self.append(self._decode_string(_escape_decode(data)))

load_string.__qualname__ = '_Unpickler.load_string'
_Unpickler.load_string = load_string
_Unpickler.dispatch[STRING[0]] = load_string
del load_string
