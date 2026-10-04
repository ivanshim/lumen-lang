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
