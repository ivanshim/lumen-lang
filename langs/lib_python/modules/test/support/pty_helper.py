"""
Helper to run a script in a pseudo-terminal.

This is CPython v3.14.8's Lib/test/support/pty_helper.py with one
change: `import selectors` is deferred into run_pty, because this
runtime carries no selectors module. Nothing here hides that: a call
to run_pty still asks for selectors (and for pty.openpty, which this
runtime refuses honestly) and gets the ImportError the missing module
raises. The import only ever happens behind readline, which this
runtime does not have, so the readline completion tests that use
run_pty step aside through CPython's own skip before reaching it.
"""

import os
import subprocess
import sys
from contextlib import ExitStack
from errno import EIO

from test.support.import_helper import import_module

def run_pty(script, input=b"dummy input\r", env=None):
    import selectors
    pty = import_module('pty')
    output = bytearray()
    [master, slave] = pty.openpty()
    args = (sys.executable, '-c', script)

    # Isolate readline from personal init files by setting INPUTRC
    # to an empty file. See also GH-142353.
    if env is None:
        env = {**os.environ.copy(), "INPUTRC": os.devnull}
    else:
        env.setdefault("INPUTRC", os.devnull)

    proc = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave, env=env)
    os.close(slave)
    with ExitStack() as cleanup:
        cleanup.enter_context(proc)
        def terminate(proc):
            try:
                proc.terminate()
            except ProcessLookupError:
                # Workaround for Open/Net BSD bug (Issue 16762)
                pass
        cleanup.callback(terminate, proc)
        cleanup.callback(os.close, master)
        # Avoid using DefaultSelector and PollSelector. Kqueue() does not
        # work with pseudo-terminals on OS X < 10.9 (Issue 20365) and Open
        # BSD (Issue 20667). Poll() does not work with OS X 10.6 or 10.4
        # either (Issue 20472). Hopefully the file descriptor is low enough
        # to use with select().
        sel = cleanup.enter_context(selectors.SelectSelector())
        sel.register(master, selectors.EVENT_READ | selectors.EVENT_WRITE)
        os.set_blocking(master, False)
        while True:
            for [_, events] in sel.select():
                if events & selectors.EVENT_READ:
                    try:
                        chunk = os.read(master, 0x10000)
                    except OSError as err:
                        # Linux raises EIO when slave is closed (Issue 5380)
                        if err.errno != EIO:
                            raise
                        chunk = b""
                    if not chunk:
                        return output
                    output.extend(chunk)
                if events & selectors.EVENT_WRITE:
                    try:
                        input = input[os.write(master, input):]
                    except OSError as err:
                        # Apparently EIO means the slave was closed
                        if err.errno != EIO:
                            raise
                        input = b""  # Stop writing
                    if not input:
                        sel.modify(master, selectors.EVENT_READ)


######################################################################
## Fake stdin (for testing interactive debugging)
######################################################################

class FakeInput:
    """
    A fake input stream for pdb's interactive debugger.  Whenever a
    line is read, print it (to simulate the user typing it), and then
    return it.  The set of lines to return is specified in the
    constructor; they should not have trailing newlines.
    """
    def __init__(self, lines):
        self.lines = lines

    def readline(self):
        line = self.lines.pop(0)
        print(line)
        return line + '\n'
