# tests/python/test_grammar.py:1495
if False:
    def test_comparison_is_literal(self):
        def check(test, msg):
            self.check_syntax_warning(test, msg)

        check('x is 1', '"is" with \'int\' literal')
        check('x is "thing"', '"is" with \'str\' literal')
        check('1 is x', '"is" with \'int\' literal')
        check('x is y is 1', '"is" with \'int\' literal')
        check('x is not 1', '"is not" with \'int\' literal')
        check('x is not (1, 2)', '"is not" with \'tuple\' literal')
        check('(1, 2) is not x', '"is not" with \'tuple\' literal')

        check('None is 1', '"is" with \'int\' literal')
        check('1 is None', '"is" with \'int\' literal')

        check('x == 3 is y', '"is" with \'int\' literal')
        check('x == "thing" is y', '"is" with \'str\' literal')

        with warnings.catch_warnings():
            warnings.simplefilter('error', SyntaxWarning)
            compile('x is None', '<testcase>', 'exec')
            compile('x is False', '<testcase>', 'exec')
            compile('x is True', '<testcase>', 'exec')
            compile('x is ...', '<testcase>', 'exec')
            compile('None is x', '<testcase>', 'exec')
            compile('False is x', '<testcase>', 'exec')
            compile('True is x', '<testcase>', 'exec')
            compile('... is x', '<testcase>', 'exec')


print('read')
