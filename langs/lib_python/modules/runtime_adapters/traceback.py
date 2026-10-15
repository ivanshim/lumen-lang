_clear_frames_python = clear_frames
_clear_frames_native = __trace_native__

def clear_frames(tb):
    if not _clear_frames_native(10, tb):
        _clear_frames_python(tb)

_should_show_carets_python = StackSummary._should_show_carets

# Keep precise runtime spans when the optional statement AST is unavailable.
def _should_show_carets_native(self, start_offset, end_offset, all_lines, anchors):
    try:
        return _should_show_carets_python(self, start_offset, end_offset, all_lines, anchors)
    except NotImplementedError:
        return True

StackSummary._should_show_carets = _should_show_carets_native
