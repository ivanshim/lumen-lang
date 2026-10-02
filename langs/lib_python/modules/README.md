# Python library sources

The integrated suite pin is CPython `v3.14.8`. The following library files are
copied unchanged, apart from one provenance and PSF licence header: enum,
pydoc, __future__, pkgutil, reprlib, token, keyword, inspect, linecache,
tokenize, functools, bisect, heapq, copyreg, posixpath, fnmatch, shlex, and the _pyrepl package initializer and pager.
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
currently buffers input eagerly; full incremental decoding and opaque seek
cookies remain outside its supported surface. Native directory-file
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

Frame, C-API, dataclass, and pickle limitations remain visible as failures or
honest skips in the reference suite. No reference test is rewritten to conceal
a runtime limitation.
