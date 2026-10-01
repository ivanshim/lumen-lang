# Source: CPython v3.14.8, Lib/contextvars.py; PSF License.
import _collections_abc
from _contextvars import Context, ContextVar, Token, copy_context


__all__ = ('Context', 'ContextVar', 'Token', 'copy_context')


_collections_abc.Mapping.register(Context)
