# CPython Lib/contextvars.py from v3.14.8 / 8e6e75d9102e, unchanged below.
# Copyright Python Software Foundation; PSF License (tests/python/LICENSE).
import _collections_abc
from _contextvars import Context, ContextVar, Token, copy_context


__all__ = ('Context', 'ContextVar', 'Token', 'copy_context')


_collections_abc.Mapping.register(Context)
