# Runtime bridges for the supported interpreter configuration and display APIs.
import os

# Read the debug-range setting that the kernels use for position records.
def config_get(name):
    if name != 'code_debug_ranges':
        raise NotImplementedError('this interpreter configuration field is unavailable')
    setting = os.environ.get('PYTHONNODEBUGRANGES', '0')
    try:
        disabled = int(setting or '0') != 0
    except ValueError:
        disabled = False
    return not disabled

# Display an actual exception through the standard formatter and stderr policy.
def exception_print(exception):
    import sys
    import traceback
    import _colorize
    if not isinstance(exception, BaseException):
        raise TypeError('an exception instance is required')
    colorize = _colorize.can_colorize(file=sys.stderr)
    traceback.TracebackException.from_exception(exception).print(file=sys.stderr, colorize=colorize)
