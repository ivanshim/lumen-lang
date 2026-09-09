if False:
    class TestPy2MigrationHint(unittest.TestCase):
        """Test that correct hint is produced analogous to Python3 syntax,
        if print statement is executed as in Python 2.
        """
    
        def test_normal_string(self):
            python2_print_str = 'print "Hello World"'
            with self.assertRaises(SyntaxError) as context:
                exec(python2_print_str)
    
            self.assertIn("Missing parentheses in call to 'print'. Did you mean print(...)",
                    str(context.exception))
    
        def test_string_with_soft_space(self):
            python2_print_str = 'print "Hello World",'
            with self.assertRaises(SyntaxError) as context:
                exec(python2_print_str)
    
            self.assertIn("Missing parentheses in call to 'print'. Did you mean print(...)",
                    str(context.exception))
    
        def test_string_with_excessive_whitespace(self):
            python2_print_str = 'print  "Hello World", '
            with self.assertRaises(SyntaxError) as context:
                exec(python2_print_str)
    
            self.assertIn("Missing parentheses in call to 'print'. Did you mean print(...)",
                    str(context.exception))
    
        def test_string_with_leading_whitespace(self):
            python2_print_str = '''if 1:
                print "Hello World"
            '''
            with self.assertRaises(SyntaxError) as context:
                exec(python2_print_str)
    
            self.assertIn("Missing parentheses in call to 'print'. Did you mean print(...)",
                    str(context.exception))
    
        # bpo-32685: Suggestions for print statement should be proper when
        # it is in the same line as the header of a compound statement
        # and/or followed by a semicolon
        def test_string_with_semicolon(self):
            python2_print_str = 'print p;'
            with self.assertRaises(SyntaxError) as context:
                exec(python2_print_str)
    
            self.assertIn("Missing parentheses in call to 'print'. Did you mean print(...)",
                    str(context.exception))
    
        def test_string_in_loop_on_same_line(self):
            python2_print_str = 'for i in s: print i'
            with self.assertRaises(SyntaxError) as context:
                exec(python2_print_str)
    
            self.assertIn("Missing parentheses in call to 'print'. Did you mean print(...)",
                    str(context.exception))
    
    
print("read")
