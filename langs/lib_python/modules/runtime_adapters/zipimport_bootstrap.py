# Expose the source bootstrap under the names used by the upstream zip loader.
import importlib as _source_importlib
import sys as _source_sys
_source_sys.modules.setdefault('_frozen_importlib', _source_importlib._bootstrap)
_source_sys.modules.setdefault('_frozen_importlib_external', _source_importlib._bootstrap_external)
