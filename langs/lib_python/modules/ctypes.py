# The foreign-function interface is unavailable in this runtime. Fail the
# optional import instead of advertising a module with no usable operations.
raise ImportError('ctypes requires a native foreign-function interface')
