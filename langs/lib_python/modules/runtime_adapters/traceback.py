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

# A recursion shows one source line for every frame, and each frame asks the
# same two questions of it again: where its anchors stand, and whether carets
# are worth drawing. Both depend on the text and offsets alone, so the answers
# for recent texts are kept (a fault is not kept, and goes up as it did).
_caret_anchors_computed = _extract_caret_anchors_from_line_segment
_caret_anchors_kept = {}

def _extract_caret_anchors_from_line_segment(segment):
    if segment in _caret_anchors_kept:
        return _caret_anchors_kept[segment]
    found = _caret_anchors_computed(segment)
    if len(_caret_anchors_kept) >= 256:
        _caret_anchors_kept.clear()
    _caret_anchors_kept[segment] = found
    return found

_carets_shown_computed = StackSummary._should_show_carets
_carets_shown_kept = {}

def _should_show_carets(self, start_offset, end_offset, all_lines, anchors):
    key = (start_offset, end_offset, tuple(all_lines), anchors)
    shown = _carets_shown_kept.get(key)
    if shown is None:
        shown = _carets_shown_computed(self, start_offset, end_offset, all_lines, anchors)
        if len(_carets_shown_kept) >= 256:
            _carets_shown_kept.clear()
        _carets_shown_kept[key] = shown
    return shown

_should_show_carets.__qualname__ = 'StackSummary._should_show_carets'
StackSummary._should_show_carets = _should_show_carets
