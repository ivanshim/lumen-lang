# Source: CPython Lib/contextvars.py, commit 3b564385e4c9.
# Licensed under the PSF License; see tests/python/LICENSE.
import _collections_abc
from _contextvars import Context, ContextVar, Token, copy_context


__all__ = ('Context', 'ContextVar', 'Token', 'copy_context')


_collections_abc.Mapping.register(Context)
