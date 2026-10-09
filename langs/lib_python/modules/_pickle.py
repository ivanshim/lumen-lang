"""Python implementation of the pickle accelerator interface.

Serialization uses the released pickle module's pure Python engines.
"""
from _picklebuffer import PickleBuffer

__all__ = ['Pickler', 'Unpickler', 'PickleError', 'PicklingError',
           'UnpicklingError', 'PickleBuffer', 'dump', 'dumps', 'load', 'loads']


def __getattr__(name):
    # Lazy binding avoids pickle's optional accelerator import cycle.
    import pickle
    engines = {'Pickler': '_Pickler', 'Unpickler': '_Unpickler',
               'dump': '_dump', 'dumps': '_dumps',
               'load': '_load', 'loads': '_loads'}
    if name in engines:
        return getattr(pickle, engines[name])
    if name in ('PickleError', 'PicklingError', 'UnpicklingError'):
        return getattr(pickle, name)
    raise AttributeError("module '_pickle' has no attribute %r" % name)
