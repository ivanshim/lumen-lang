# Stub: there is no built decimal arithmetic behind the decimal module
# here, and a guarded import must take the absent-module path, which a
# module that is not there at all does not give it.
_fault = ImportError()
_fault.message = '_decimal is not available'
raise _fault
