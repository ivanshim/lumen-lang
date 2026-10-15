# Bind the compatibility name after the source bootstrap has initialized.
import sys
_bootstrap_module = sys.modules.get('importlib._bootstrap')
if _bootstrap_module is None:
    raise ImportError("source bootstrap is not initialized")
sys.modules[__name__] = _bootstrap_module
