# From CPython v3.14.8 (8e6e75d9102e), Lib/test/typinganndata/mod_generics_cache.py; PSF License.
"""Module for testing the behavior of generics across different modules."""

from typing import TypeVar, Generic, Optional, TypeAliasType

default_a: Optional['A'] = None
default_b: Optional['B'] = None

T = TypeVar('T')


class A(Generic[T]):
    some_b: 'B'


class B(Generic[T]):
    class A(Generic[T]):
        pass

    my_inner_a1: 'B.A'
    my_inner_a2: A
    my_outer_a: 'A'  # unless somebody calls get_type_hints with localns=B.__dict__

type Alias = int
OldStyle = TypeAliasType("OldStyle", int)
