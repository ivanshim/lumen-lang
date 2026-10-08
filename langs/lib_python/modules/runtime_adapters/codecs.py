# Expose the buffer helper the runtime's native struct readers call.
# The canonical codecs.py remains the unchanged CPython source.
def _escape_buffer(source):
    import builtins
    try:
        with builtins._buffer_view(source, 0) as view:
            if not view.c_contiguous:
                raise BufferError('memoryview: underlying buffer is not C-contiguous')
            return view.tobytes()
    except TypeError as exc:
        message = str(exc)
        if message.startswith('memoryview: '):
            message = message[12:]
        raise TypeError(message) from None
