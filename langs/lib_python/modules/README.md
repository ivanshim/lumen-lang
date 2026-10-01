# Python library sources

The enum support uses CPython commit `3b564385e4c9`. `enum.py`, `pydoc.py`,
`__future__.py`, `pkgutil.py`, `reprlib.py`, `token.py`, and `keyword.py`
retain the upstream source below a PSF provenance comment. `linecache.py`
retains the upstream implementation with uncached filesystem source reading
when os.stat is unavailable, without inventing timestamps.

The import execution remains in the kernels. The full CPython bootstrap
requires `_imp`, frozen modules, native import locks, and runtime installation
hooks that these kernels do not expose. `importlib._bootstrap` therefore
retains the upstream `ModuleSpec`, `spec_from_loader`, `_spec_from_module`,
`_init_module_attrs`, `module_from_spec`, `_load_unlocked`, and `_resolve_name`
implementations. Its module creation, diagnostic output, frame-call wrapper,
and single-threaded `_load` entry use the existing runtime facilities.
`_bootstrap_external` retains `spec_from_file_location` and provides source
loaders and filesystem finders. CPython bytecode and extension execution
raise `ImportError`. Embedded modules continue through the native importer;
this is not a replacement for CPython's complete import framework.

`tokenize.py` uses CPython v3.11.0's pure-Python tokenizer because the newer
source requires the native `_tokenize` implementation. Its `open` helper uses
the runtime's text streams after the upstream encoding detection, since
`io.TextIOWrapper` is unavailable. `sysconfig` describes the embedded library
paths; it supplies no CPython build configuration. `_thread` exposes the
existing thread identifier, without advertising native threading support.

Member, module, documentation, and source-comment discovery in `inspect`,
wrapper metadata and single
dispatch in `functools`, and the support helpers needed by the enum suite
reuse upstream Python implementations. Signature, Parameter, and bound
argument models also retain upstream implementations, using ordered builtin
dictionaries in Signature rather than an OrderedDict subclass. Annotation formatting
uses the 3.11 implementation because ForwardRef is unavailable; native text
signatures, partial-object signatures, and frame inspection remain unsupported.
Complete dataclass behavior and the remaining container and pickle
protocols retain their runtime limitations; the tests report them.
The changes to `abc`, `collections`, `datetime`, `os`, `re`, `sys`, and the documented `itertools.chain` recipe are
limited to the dependencies required by these library paths.
