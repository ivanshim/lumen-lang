# Python library sources

The integrated suite pin is CPython `v3.14.8`. The following library files are
copied unchanged, apart from one provenance and PSF licence header: enum, netrc, fileinput,
pydoc, __future__, pkgutil, reprlib, token, keyword, inspect, linecache, mimetypes,
tokenize, functools, bisect, heapq, copyreg, genericpath, posixpath, stat, fnmatch, shlex, http.cookies, unittest.util, and the _pyrepl package initializer and pager.
Their upstream bodies are checked byte for byte during source auditing.

Native C facilities are modeled separately. `_tokenize` supplies the pinned
TokenizerIter interface; its Python scanner derives from CPython's older
pure-Python tokenizer and separates interpolation tokens. The public tokenize
module is the pinned source, including its original open implementation.
The separate scanner handles ordinary tokens and interpolation conversions,
format specifications, and nested replacement fields. It is not CPython's C
lexer: interpolation with the enclosing quote reused inside a replacement
expression and some malformed-input diagnostic positions remain unsupported.
The existing io adapter supplies text streams, and the os adapter obtains
real stat fields through a Python-only runtime label. The text-stream adapter
uses the binary buffer as its only contents and cursor; construction does
not read it. ASCII, UTF-8 and single-byte sized reads use byte-position
cookies derived from the buffer position and unread input. Writes discard
read-ahead, matching TextIOWrapper update-stream behavior. Stateful codecs
and opaque decoder-state cookies remain unsupported. Native directory-file
stat operations remain unsupported and raise NotImplementedError.

The import execution remains in the kernels. The full CPython bootstrap
requires `_imp`, frozen modules, native import locks, and installation hooks
that these kernels do not expose. The partial bootstrap preserves ModuleSpec,
spec_from_loader, _spec_from_module, _init_module_attrs, module_from_spec,
_load_unlocked, and _resolve_name. Source-loader and filesystem finder adapters
provide the names used by inspect and pydoc. Bytecode and extension execution
raise ImportError. These adapters are not claimed to be unchanged CPython files.

The inherited abc, collections, datetime, doctest, importlib, os, re,
sysconfig, test.support, types, and unittest modules are runtime adapters.
Some reuse upstream functions; their complete files are not upstream copies.
The native-module models _thread, itertools, sys, and io likewise have no
corresponding pure-Python implementation of their C facilities. Their existing
behavior and the newer integrated suite inventory are retained.

`types.py` is the unchanged CPython v3.14.8 `Lib/types.py` (sha256
8c54d3d5ffc1d1204237e6c69b25c27c7b05b483128f185eeed9ba7ef2229ac2); it star-imports
`_types`, the runtime's stand-in for `Modules/_typesmodule.c`, whose names are
the runtime's own kinds (the pure fallback in `types.py` reads the same values)
including the native mapping proxy, plus helpers for unions and subscripted
kinds. `CapsuleType` is absent because there is no runtime C-API capsule kind. `_datetime` has no C accelerator behind
it: it re-exports the classes of `_pydatetime`, the very objects `datetime`
exposes, and has no `datetime_CAPI` capsule.

Frame, C-API, dataclass, and pickle limitations remain visible as failures or
honest skips in the reference suite. No reference test is rewritten to conceal
a runtime limitation.

`_ordering` preserves the inherited stable-ordering runtime helper outside the
unchanged upstream `heapq` source. The existing `pprint` and `statistics` adapters
import it directly; their algorithms are unchanged.

Compiled module code can be used with the native function constructor and
executes through the existing code reader, retaining its code and globals.
That constructor currently accepts positional code, globals and an optional
name for module code; the remaining optional constructor arguments are not
yet supported on this code representation. Generator expression operands
retain their `.0` binding and are checked as iterators when resumed.

`concurrent/interpreters/__init__.py` is a byte-for-byte copy of CPython
v3.14.8 `Lib/concurrent/interpreters/__init__.py`. Importing it requires the
absent `_interpreters` backend and propagates `ModuleNotFoundError`; callers
can detect the unavailable facility through the standard import exception.
