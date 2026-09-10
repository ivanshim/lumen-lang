# tests/python/test_decorators.py:208
if False:
    def test_double(self):
        class C(object):
            @funcattrs(abc=1, xyz="haha")
            @funcattrs(booh=42)
            def foo(self): return 42
        self.assertEqual(C().foo(), 42)
        self.assertEqual(C.foo.abc, 1)
        self.assertEqual(C.foo.xyz, "haha")
        self.assertEqual(C.foo.booh, 42)


print('read')
