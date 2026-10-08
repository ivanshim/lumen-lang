# CPython's frozen external import machinery, under the name its
# startup gives it; this runtime carries it as
# importlib._bootstrap_external. The wiring the reference performs when
# the frozen name is absent is performed here, so the two bootstrap
# modules stand bound to one another either way.
import sys
import importlib._bootstrap as _bootstrap
import importlib._bootstrap_external as _bootstrap_external
_bootstrap_external._set_bootstrap_module(_bootstrap)
_bootstrap._bootstrap_external = _bootstrap_external
sys.modules[__name__] = _bootstrap_external
