# tests/python/test_with.py:219
if False:
    def testEnterThrows(self):
        class EnterThrows(object):
            def __enter__(self):
                raise RuntimeError("Enter threw")
            def __exit__(self, *args):
                pass

        def shouldThrow():
            ct = EnterThrows()
            self.foo = None
            # Ruff complains that we're redefining `self.foo` here,
            # but the whole point of the test is to check that `self.foo`
            # is *not* redefined (because `__enter__` raises)
            with ct as self.foo:  # noqa: F811
                pass
        self.assertRaises(RuntimeError, shouldThrow)
        self.assertEqual(self.foo, None)


print('read')
