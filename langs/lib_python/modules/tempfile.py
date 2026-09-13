# Where a program would put a file it means to throw away.
#
# Every one of CPython's temporary things ends in a directory being made
# or a file being opened. This runtime makes no directory -- os.mkdir
# refuses -- and offers no builtin open, so none of them can be made
# here. What is left that can be answered truthfully is where the host
# says such things belong and what they are usually called, and those
# two are below. The rest refuse and say which missing piece stops them,
# rather than handing back a name that no directory stands behind.
import os

__all__ = ['NamedTemporaryFile', 'TemporaryFile', 'SpooledTemporaryFile',
           'TemporaryDirectory', 'mkstemp', 'mkdtemp', 'mktemp', 'gettempdir',
           'gettempprefix', 'tempdir', 'template']

template = 'tmp'
tempdir = None


def gettempprefix():
    return template


def gettempdir():
    # The host names the place in the environment, and the first of the
    # three usual names that is set wins, as CPython's own search does.
    # Nothing is made or written to check it.
    if tempdir is not None:
        return tempdir
    places = os.environ
    for name in ['TMPDIR', 'TEMP', 'TMP']:
        if name in places:
            return places[name]
    return '/tmp'


def mkdtemp(suffix=None, prefix=None, dir=None):
    raise 'NotImplementedError: tempfile.mkdtemp needs os.mkdir, which this runtime does not carry'


def mkstemp(suffix=None, prefix=None, dir=None, text=False):
    raise 'NotImplementedError: tempfile.mkstemp needs a file to be created and opened, which this runtime does not carry'


def mktemp(suffix='', prefix=template, dir=None):
    raise 'NotImplementedError: tempfile.mktemp would name a file this runtime cannot then create'


def NamedTemporaryFile(*args, **keywords):
    raise 'NotImplementedError: tempfile.NamedTemporaryFile needs a file to be created and opened, which this runtime does not carry'


def TemporaryFile(*args, **keywords):
    raise 'NotImplementedError: tempfile.TemporaryFile needs a file to be created and opened, which this runtime does not carry'


def SpooledTemporaryFile(*args, **keywords):
    # CPython's keeps the writing in memory until it grows past a limit
    # and then moves it into a real file. The first half could be an
    # in-memory stream here, but a program that writes past the limit
    # would go on writing into memory and never learn that the move did
    # not happen, so the whole of it refuses.
    raise 'NotImplementedError: tempfile.SpooledTemporaryFile cannot roll over into a file, which this runtime does not carry'


def TemporaryDirectory(*args, **keywords):
    raise 'NotImplementedError: tempfile.TemporaryDirectory needs os.mkdir, which this runtime does not carry'
