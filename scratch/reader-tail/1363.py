# tests/python/test_with.py:197
if False:
    def assertRaisesSyntaxError(self, codestr):
        def shouldRaiseSyntaxError(s):
            compile(s, '', 'single')
        self.assertRaises(SyntaxError, shouldRaiseSyntaxError, codestr)


print('read')
