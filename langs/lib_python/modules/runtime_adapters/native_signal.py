# Signals kept pending and taken up between one statement and the next.
#
# The numbers and names are the Linux ones. A handler is named with
# signal() and asked back with getsignal(); raise_signal() leaves a
# signal pending, and the run takes it up at the next statement's edge,
# running the handler named for it — or raising KeyboardInterrupt where
# the interrupt signal still goes its own way.
SIGHUP = 1
SIGINT = 2
SIGQUIT = 3
SIGILL = 4
SIGTRAP = 5
SIGABRT = 6
SIGBUS = 7
SIGFPE = 8
SIGKILL = 9
SIGUSR1 = 10
SIGSEGV = 11
SIGUSR2 = 12
SIGPIPE = 13
SIGALRM = 14
SIGTERM = 15
SIGSTKFLT = 16
SIGCHLD = 17
SIGCONT = 18
SIGSTOP = 19
SIGTSTP = 20
SIGTTIN = 21
SIGTTOU = 22
SIGURG = 23
SIGXCPU = 24
SIGXFSZ = 25
SIGVTALRM = 26
SIGPROF = 27
SIGWINCH = 28
SIGIO = 29
SIGPWR = 30
SIGSYS = 31
SIGRTMIN = 34
SIGRTMAX = 64
SIGCLD = SIGCHLD
SIGPOLL = SIGIO
SIGIOT = SIGABRT

NSIG = 65
SIG_DFL = 0
SIG_IGN = 1

_names = {
    1: 'SIGHUP',
    2: 'SIGINT',
    3: 'SIGQUIT',
    4: 'SIGILL',
    5: 'SIGTRAP',
    6: 'SIGABRT',
    7: 'SIGBUS',
    8: 'SIGFPE',
    9: 'SIGKILL',
    10: 'SIGUSR1',
    11: 'SIGSEGV',
    12: 'SIGUSR2',
    13: 'SIGPIPE',
    14: 'SIGALRM',
    15: 'SIGTERM',
    16: 'SIGSTKFLT',
    17: 'SIGCHLD',
    18: 'SIGCONT',
    19: 'SIGSTOP',
    20: 'SIGTSTP',
    21: 'SIGTTIN',
    22: 'SIGTTOU',
    23: 'SIGURG',
    24: 'SIGXCPU',
    25: 'SIGXFSZ',
    26: 'SIGVTALRM',
    27: 'SIGPROF',
    28: 'SIGWINCH',
    29: 'SIGIO',
    30: 'SIGPWR',
    31: 'SIGSYS',
    34: 'SIGRTMIN',
    64: 'SIGRTMAX',
}

_descriptions = {
    1: 'Hangup',
    2: 'Interrupt',
    3: 'Quit',
    4: 'Illegal instruction',
    5: 'Trace/breakpoint trap',
    6: 'Aborted',
    7: 'Bus error',
    8: 'Floating point exception',
    9: 'Killed',
    10: 'User defined signal 1',
    11: 'Segmentation fault',
    12: 'User defined signal 2',
    13: 'Broken pipe',
    14: 'Alarm clock',
    15: 'Terminated',
    16: 'Stack fault',
    17: 'Child exited',
    18: 'Continued',
    19: 'Stopped (signal)',
    20: 'Stopped',
    21: 'Stopped (tty input)',
    22: 'Stopped (tty output)',
    23: 'Urgent I/O condition',
    24: 'CPU time limit exceeded',
    25: 'File size limit exceeded',
    26: 'Virtual timer expired',
    27: 'Profiling timer expired',
    28: 'Window changed',
    29: 'I/O possible',
    30: 'Power failure',
    31: 'Bad system call',
    34: 'Real-time signal 0',
    64: 'Real-time signal 30',
}

def default_int_handler(signalnum, frame):
    raise KeyboardInterrupt

def _checked_number(signalnum):
    # IntEnum members compare as ints but are not the kernel's own integers.
    from posix import _cint
    try:
        signalnum = _cint(signalnum)
    except (TypeError, ValueError):
        raise TypeError('an integer is required')
    if signalnum < 1 or signalnum >= NSIG:
        raise ValueError('signal number out of range')
    return signalnum

def signal(signalnum, handler):
    signalnum = _checked_number(signalnum)
    if isinstance(handler, int):
        from posix import _cint
        handler = _cint(handler)
    # 0 and 1 are SIG_DFL and SIG_IGN. After IntEnum conversion the
    # public names are members, so the kernel path compares the numbers.
    if handler != 0 and handler != 1 and not callable(handler):
        raise TypeError('signal handler must be signal.SIG_IGN, signal.SIG_DFL, or a callable')
    previous = __signal(0, signalnum, handler)
    if previous is None:
        if signalnum == 2:
            return default_int_handler
        return SIG_DFL
    return previous

def getsignal(signalnum):
    signalnum = _checked_number(signalnum)
    handler = __signal(1, signalnum)
    if handler is None:
        if signalnum == 2:
            return default_int_handler
        return SIG_DFL
    return handler

def raise_signal(signalnum):
    signalnum = _checked_number(signalnum)
    # The number is left pending as this wrapper's last word: no
    # statement's edge stands between here and the caller it returns
    # to, so the signal is taken up from there, never from inside the
    # raising call.
    __signal(2, signalnum)

def strsignal(signalnum):
    if isinstance(signalnum, int) and type(signalnum) is not int:
        from posix import _cint
        signalnum = _cint(signalnum)
    if signalnum in _descriptions:
        return _descriptions[signalnum]
    if signalnum < 1 or signalnum >= NSIG:
        raise 'ValueError: signal number out of range'
    return None

def valid_signals():
    result = set()
    for number in _names:
        result.add(number)
    return result

def alarm(seconds):
    import operator
    return __signal(3, operator.index(seconds))

def pause():
    return __signal(4)


ITIMER_REAL, ITIMER_VIRTUAL, ITIMER_PROF = 0, 1, 2


class ItimerError(OSError):
    pass


# Convert timer seconds into positive microseconds, rounding away from zero.
def _timer_us(value):
    import math
    import operator
    if not isinstance(value, float):
        value = operator.index(value)
    if math.isnan(value):
        raise ValueError('Invalid value NaN (not a number)')
    if value < 0:
        raise ItimerError(22, 'Invalid argument')
    if value >= 9223372036854:
        raise OverflowError('timestamp out of range for platform time_t')
    return math.ceil(value * 1000000)


# Set one of the host's real, virtual, or profiling interval timers.
def setitimer(which, seconds, interval=0.0, /):
    from posix import _call, _cint
    try:
        previous = _call('setitimer', _cint(which), _timer_us(seconds), _timer_us(interval))
    except OSError as error:
        raise ItimerError(*error.args)
    return (previous[0] / 1000000, previous[1] / 1000000)


# Return the remaining delay and repetition interval in seconds.
def getitimer(which, /):
    from posix import _call, _cint
    try:
        current = _call('getitimer', _cint(which))
    except OSError as error:
        raise ItimerError(*error.args)
    return (current[0] / 1000000, current[1] / 1000000)


# The public constants are IntEnum members, as CPython's signal module
# builds them. The kernel still receives plain integers.
from enum import IntEnum as _IntEnum

def _int_to_enum(value, enum_klass):
    if not isinstance(value, int):
        return value
    try:
        return enum_klass(value)
    except ValueError:
        return value

def _enum_to_int(value):
    from posix import _cint
    try:
        return _cint(value)
    except (TypeError, ValueError, OverflowError):
        return value

_IntEnum._convert_(
        'Signals', __name__,
        lambda name:
            name.isupper()
            and (name.startswith('SIG') and not name.startswith('SIG_'))
            or name.startswith('CTRL_'))

_IntEnum._convert_(
        'Handlers', __name__,
        lambda name: name in ('SIG_DFL', 'SIG_IGN'))

_raw_signal = signal
_raw_getsignal = getsignal
_raw_valid_signals = valid_signals

def signal(signalnum, handler):
    previous = _raw_signal(_enum_to_int(signalnum), _enum_to_int(handler))
    return _int_to_enum(previous, Handlers)

def getsignal(signalnum):
    handler = _raw_getsignal(_enum_to_int(signalnum))
    return _int_to_enum(handler, Handlers)

def valid_signals():
    return {_int_to_enum(number, Signals) for number in _raw_valid_signals()}

def _wakeup_error(number):
    import os
    import sys
    sys._report_unraisable(OSError(number, os.strerror(number)), None,
                           None, 'signal')


def set_wakeup_fd(fd, /, *, warn_on_full_buffer=True):
    import os
    from posix import _cint
    fd = _cint(fd)
    __signal(6)
    if fd != -1:
        if fd < 0:
            raise ValueError('invalid fd')
        if os.get_blocking(fd):
            raise ValueError('the fd %s must be in non-blocking mode' % fd)
    return __signal(5, fd, int(bool(warn_on_full_buffer)), _wakeup_error)
