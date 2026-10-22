# Pseudo-terminals.
#
# forkpty opens a terminal pair and forks a session leader onto the
# slave. openpty and spawn are not carried separately: a pair with
# nobody sitting on the far end would name a terminal that is not there.
import os

STDIN_FILENO = 0
STDOUT_FILENO = 1
STDERR_FILENO = 2

CHILD = 0


def openpty():
    raise OSError('pty.openpty needs a terminal device pair, which this runtime cannot open')


def fork():
    """Fork, and put the child on a new controlling terminal.

    Returns (pid, master_fd). OSError when the host cannot fork.
    """
    return os.forkpty()


def spawn(argv, master_read=None, stdin_read=None):
    raise OSError('pty.spawn needs to fork a process onto a terminal, which this runtime cannot do')
