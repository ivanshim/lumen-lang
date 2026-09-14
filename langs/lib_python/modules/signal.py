# The Linux signal numbers. Nothing here can deliver a signal: this
# runtime has no handler dispatch, so the functions that would install
# one refuse rather than accept a handler that could never run.
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

def strsignal(signalnum):
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

def getsignal(signalnum):
    if signalnum not in _names:
        raise 'ValueError: signal number out of range'
    return SIG_DFL

def signal(signalnum, handler):
    raise 'NotImplementedError: signal handlers cannot be delivered here'

def alarm(seconds):
    raise 'NotImplementedError: signal.alarm cannot deliver SIGALRM here'

def raise_signal(signalnum):
    raise 'NotImplementedError: signals cannot be raised here'

def pause():
    raise 'NotImplementedError: no signal can arrive to end a pause'

def default_int_handler(signalnum, frame):
    raise 'KeyboardInterrupt'
