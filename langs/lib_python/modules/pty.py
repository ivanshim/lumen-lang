# Pseudo-terminals, which this runtime has none of.
#
# Everything CPython's pty does rests on opening a pair of terminal
# devices and forking a process to sit on the far end of them. This
# runtime opens no device and forks no process, so not one of the three
# calls below can be carried out, and each says so rather than handing
# back a number that names nothing.
#
# The module is here all the same because a program that asks for it
# inside a try and means to go on without it cannot go on here: the
# fault raised for a module that is missing is not caught by a guard
# written for an import that fails. So the module is found, and a
# program that only names it gets as far as the first call.

STDIN_FILENO = 0
STDOUT_FILENO = 1
STDERR_FILENO = 2

CHILD = 0


def openpty():
    raise 'NotImplementedError: pty.openpty needs a terminal device pair, which this runtime cannot open'


def fork():
    raise 'NotImplementedError: pty.fork needs to fork a process, which this runtime cannot do'


def spawn(argv, master_read=None, stdin_read=None):
    raise 'NotImplementedError: pty.spawn needs to fork a process onto a terminal, which this runtime cannot do'
