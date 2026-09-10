# tests/python/test_grammar.py:183
if False:
        def test_string_literals(self):
            x = ''; y = ""; self.assertTrue(len(x) == 0 and x == y)
            x = '\''; y = "'"; self.assertTrue(len(x) == 1 and x == y and ord(x) == 39)
            x = '"'; y = "\""; self.assertTrue(len(x) == 1 and x == y and ord(x) == 34)
            x = "doesn't \"shrink\" does it"
            y = 'doesn\'t "shrink" does it'
            self.assertTrue(len(x) == 24 and x == y)
            x = "does \"shrink\" doesn't it"
            y = 'does "shrink" doesn\'t it'
            self.assertTrue(len(x) == 24 and x == y)
            x = """
    The "quick"
    brown fox
    jumps over
    the 'lazy' dog.
    """
            y = '\nThe "quick"\nbrown fox\njumps over\nthe \'lazy\' dog.\n'
            self.assertEqual(x, y)
            y = '''
    The "quick"
    brown fox
    jumps over
    the 'lazy' dog.
    '''
            self.assertEqual(x, y)
            y = "\n\
    The \"quick\"\n\
    brown fox\n\
    jumps over\n\
    the 'lazy' dog.\n\
    "
            self.assertEqual(x, y)
            y = '\n\
    The \"quick\"\n\
    brown fox\n\
    jumps over\n\
    the \'lazy\' dog.\n\
    '
            self.assertEqual(x, y)


print('read')
