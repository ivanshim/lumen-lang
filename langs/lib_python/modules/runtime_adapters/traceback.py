_clear_frames_python = clear_frames
_clear_frames_native = __trace_native__

def clear_frames(tb):
    if not _clear_frames_native(10, tb):
        _clear_frames_python(tb)


# A missing syntax-tree capability cannot prevent reporting the original error.
_should_show_carets_python = StackSummary._should_show_carets

def _should_show_carets_available(self, start_offset, end_offset, all_lines, anchors):
    try:
        return _should_show_carets_python(self, start_offset, end_offset, all_lines, anchors)
    except NotImplementedError:
        return False

StackSummary._should_show_carets = _should_show_carets_available
