# Assert that the guarded operations emit no resource warnings.
@contextlib.contextmanager
def check_no_resource_warning(testcase):
    """Context manager to check that no ResourceWarning is emitted.

    Usage:

        with check_no_resource_warning(self):
            f = open(...)
            ...
            del f

    You must remove the object which may emit ResourceWarning before
    the end of the context manager.
    """
    with check_no_warnings(testcase, category=ResourceWarning, force_gc=True):
        yield
