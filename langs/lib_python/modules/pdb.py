"""Debugger entry points; tracing interaction is not implemented yet."""
import sys


class Pdb:
    """Prepare debugger state and report unsupported tracing honestly."""

    def __init__(self, *, stdout=None, nosigint=False):
        self.stdout = sys.stdout if stdout is None else stdout
        self.nosigint = nosigint
        self.reset()

    def reset(self):
        self.botframe = None

    def set_trace(self, frame=None, *, commands=None):
        raise NotImplementedError('pdb tracing interaction is not supported')

    def set_continue(self):
        sys.settrace(None)

    def trace_dispatch(self, frame, event, arg):
        raise NotImplementedError('pdb tracing interaction is not supported')


def set_trace(*args, **kwargs):
    raise NotImplementedError('pdb tracing interaction is not supported')
