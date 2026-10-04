# Source: CPython 3b564385e4c9, Lib/os.py PathLike; PSF License.
from abc import ABC, abstractmethod
import abc
from types import GenericAlias
from _collections_abc import _check_methods
class PathLike(abc.ABC):

    """Abstract base class for implementing the file system path protocol."""

    __slots__ = ()

    @abc.abstractmethod
    def __fspath__(self):
        """Return the file system path representation of the object."""
        raise NotImplementedError

    @classmethod
    def __subclasshook__(cls, subclass):
        if cls is PathLike:
            return _check_methods(subclass, '__fspath__')
        return NotImplemented

    __class_getitem__ = classmethod(GenericAlias)
