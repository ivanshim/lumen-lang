import unittest
@unittest.skipUnless(False, "no")
def f(): pass
print(getattr(f, "__unittest_skip__", None))
