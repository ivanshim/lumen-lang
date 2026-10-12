# Source: CPython v3.14.8 Lib/_py_warnings.py option helpers; PSF License.
import sys
_wm = __import__(__name__)

class _OptionError(Exception):
    """Exception used by option processing helpers."""
    pass


# Helper to process -W options passed via sys.warnoptions
def _processoptions(args):
    for arg in args:
        try:
            _wm._setoption(arg)
        except _wm._OptionError as msg:
            print("Invalid -W option ignored:", msg, file=sys.stderr)


# Helper for _processoptions()
def _setoption(arg):
    parts = arg.split(':')
    if len(parts) > 5:
        raise _wm._OptionError("too many fields (max 5): %r" % (arg,))
    while len(parts) < 5:
        parts.append('')
    action, message, category, module, lineno = [s.strip()
                                                 for s in parts]
    action = _wm._getaction(action)
    category = _wm._getcategory(category)
    if message or module:
        import re
    if message:
        message = re.escape(message)
    if module:
        module = re.escape(module) + r'\z'
    if lineno:
        try:
            lineno = int(lineno)
            if lineno < 0:
                raise ValueError
        except (ValueError, OverflowError):
            raise _wm._OptionError("invalid lineno %r" % (lineno,)) from None
    else:
        lineno = 0
    _wm.filterwarnings(action, message, category, module, lineno)


# Helper for _setoption()
def _getaction(action):
    if not action:
        return "default"
    for a in ('default', 'always', 'all', 'ignore', 'module', 'once', 'error'):
        if a.startswith(action):
            return a
    raise _wm._OptionError("invalid action: %r" % (action,))


# Helper for _setoption()
def _getcategory(category):
    if not category:
        return Warning
    if '.' not in category:
        import builtins as m
        klass = category
    else:
        module, _, klass = category.rpartition('.')
        try:
            m = __import__(module, None, None, [klass])
        except ImportError:
            raise _wm._OptionError("invalid module name: %r" % (module,)) from None
    try:
        cat = getattr(m, klass)
    except AttributeError:
        raise _wm._OptionError("unknown warning category: %r" % (category,)) from None
    if not issubclass(cat, Warning):
        raise _wm._OptionError("invalid warning category: %r" % (category,))
    return cat


_processoptions(sys.warnoptions)
