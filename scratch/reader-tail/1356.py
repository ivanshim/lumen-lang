# tests/python/test_with.py:120
if False:
    def testNameError(self):
        def fooNotDeclared():
            with foo: pass
        self.assertRaises(NameError, fooNotDeclared)


print('read')
