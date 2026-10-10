_clear_frames_python = clear_frames
_clear_frames_native = __trace_native__

def clear_frames(tb):
    if not _clear_frames_native(10, tb):
        _clear_frames_python(tb)
