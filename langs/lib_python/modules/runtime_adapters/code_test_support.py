# Source: CPython v3.14.8 Lib/test/support/__init__.py; PSF License.
import os
import unittest

def has_no_debug_ranges():
    try:
        import _testcapi
    except ImportError:
        raise unittest.SkipTest("_testinternalcapi required")
    return not _testcapi.config_get('code_debug_ranges')


def requires_debug_ranges(reason='requires co_positions / debug_ranges'):
    try:
        skip = has_no_debug_ranges()
    except unittest.SkipTest as e:
        skip = True
        reason = e.args[0] if e.args else reason
    return unittest.skipIf(skip, reason)


def late_deletion(obj):
    """
    Keep a Python alive as long as possible.

    Create a reference cycle and store the cycle in an object deleted late in
    Python finalization. Try to keep the object alive until the very last
    garbage collection.

    The function keeps a strong reference by design. It should be called in a
    subprocess to not mark a test as "leaking a reference".
    """

    # Late CPython finalization:
    # - finalize_interp_clear()
    # - _PyInterpreterState_Clear(): Clear PyInterpreterState members
    #   (ex: codec_search_path, before_forkers)
    # - clear os.register_at_fork() callbacks
    # - clear codecs.register() callbacks

    ref_cycle = [obj]
    ref_cycle.append(ref_cycle)

    # Store a reference in PyInterpreterState.codec_search_path
    import codecs
    def search_func(encoding):
        return None
    search_func.reference = ref_cycle
    codecs.register(search_func)

    if hasattr(os, 'register_at_fork'):
        # Store a reference in PyInterpreterState.before_forkers
        def atfork_func():
            pass
        atfork_func.reference = ref_cycle
        os.register_at_fork(before=atfork_func)
