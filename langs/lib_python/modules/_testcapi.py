# Stub: guarded imports take the absent-module path.
_fault = ImportError()
_fault.message = '_testcapi is not available'
raise _fault
