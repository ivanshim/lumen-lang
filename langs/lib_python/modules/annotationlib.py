# The formats an __annotate__ function is asked for, and the one reader of
# annotations that works without one. This runtime records the names a class
# or module annotates but does not evaluate the annotations themselves, so
# only VALUE can be served; the formats that need the annotation expression
# back are refused rather than answered with a guess.

class Format:
    VALUE = 1
    VALUE_WITH_FAKE_GLOBALS = 2
    FORWARDREF = 3
    STRING = 4

_FORMAT_NAMES = {1: 'VALUE', 2: 'VALUE_WITH_FAKE_GLOBALS', 3: 'FORWARDREF', 4: 'STRING'}

def _check_format(format):
    if format not in _FORMAT_NAMES:
        raise 'ValueError: ' + str(format) + ' is not a valid Format'
    if format != Format.VALUE:
        raise 'NotImplementedError: annotationlib can only serve Format.VALUE here'

def get_annotate_from_class_namespace(obj):
    # A class body here never leaves an __annotate__ behind.
    try:
        return obj['__annotate__']
    except Exception:
        return None

def call_annotate_function(annotate, format, owner=None):
    _check_format(format)
    return annotate(format)

def call_evaluate_function(evaluate, format, owner=None):
    _check_format(format)
    return evaluate(format)

def get_annotations(obj, *, globals=None, locals=None, eval_str=False, format=1):
    _check_format(format)
    annotate = getattr(obj, '__annotate__', None)
    if annotate is not None:
        result = annotate(format)
        if result is None:
            return {}
        return dict(result)
    stored = getattr(obj, '__annotations__', None)
    if stored is None:
        return {}
    if eval_str:
        raise 'NotImplementedError: annotationlib cannot evaluate string annotations here'
    return dict(stored)
