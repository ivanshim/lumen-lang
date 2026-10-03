# Adapted from CPython Lib/annotationlib.py get_annotations at v3.14.8 / 8e6e75d9102e; PSF License.
# Resolved annotations can serve FORWARDREF without symbolic evaluation.
_value_get_annotations = get_annotations

def get_annotations(obj, *, globals=None, locals=None, eval_str=False, format=Format.VALUE):
    if format != Format.FORWARDREF:
        return _value_get_annotations(obj, globals=globals, locals=locals,
                                      eval_str=eval_str, format=format)
    if eval_str:
        raise ValueError("eval_str=True is only supported with format=Format.VALUE")
    try:
        ann = getattr(obj, '__annotations__', None)
        if ann is not None and not isinstance(ann, dict):
            raise ValueError(f"{obj!r}.__annotations__ is neither a dict nor None")
    except Exception:
        pass
    else:
        if ann is not None:
            return dict(ann)
    annotate = getattr(obj, '__annotate__', None)
    if annotate is not None:
        ann = call_annotate_function(annotate, format, owner=obj)
        if not isinstance(ann, dict):
            raise ValueError(f"{obj!r}.__annotate__ returned a non-dict")
        return dict(ann)
    ann = getattr(obj, '__annotations__', None)
    if ann is None:
        if isinstance(obj, type) or callable(obj):
            return {}
        raise TypeError(f"{obj!r} does not have annotations")
    if not isinstance(ann, dict):
        raise ValueError(f"{obj!r}.__annotations__ is neither a dict nor None")
    return dict(ann)
