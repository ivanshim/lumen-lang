if False:
    class TestPrint(unittest.TestCase):
        """Test correct operation of the print function."""
    
        def check(self, expected, args,
                  sep=NotDefined, end=NotDefined, file=NotDefined):
            # Capture sys.stdout in a StringIO.  Call print with args,
            # and with sep, end, and file, if they're defined.  Result
            # must match expected.
    
            # Look up the actual function to call, based on if sep, end,
            # and file are defined.
            fn = dispatch[(sep is not NotDefined,
                           end is not NotDefined,
                           file is not NotDefined)]
    
            with support.captured_stdout() as t:
                fn(args, sep, end, file)
    
            self.assertEqual(t.getvalue(), expected)
    
        def test_print(self):
            def x(expected, args, sep=NotDefined, end=NotDefined):
                # Run the test 2 ways: not using file, and using
                # file directed to a StringIO.
    
                self.check(expected, args, sep=sep, end=end)
    
                # When writing to a file, stdout is expected to be empty
                o = StringIO()
                self.check('', args, sep=sep, end=end, file=o)
    
                # And o will contain the expected output
                self.assertEqual(o.getvalue(), expected)
    
            x('\n', ())
            x('a\n', ('a',))
            x('None\n', (None,))
            x('1 2\n', (1, 2))
            x('1   2\n', (1, ' ', 2))
            x('1*2\n', (1, 2), sep='*')
            x('1 s', (1, 's'), end='')
            x('a\nb\n', ('a', 'b'), sep='\n')
            x('1.01', (1.0, 1), sep='', end='')
            x('1*a*1.3+', (1, 'a', 1.3), sep='*', end='+')
            x('a\n\nb\n', ('a\n', 'b'), sep='\n')
            x('\0+ +\0\n', ('\0', ' ', '\0'), sep='+')
    
            x('a\n b\n', ('a\n', 'b'))
            x('a\n b\n', ('a\n', 'b'), sep=None)
            x('a\n b\n', ('a\n', 'b'), end=None)
            x('a\n b\n', ('a\n', 'b'), sep=None, end=None)
    
            x('*\n', (ClassWith__str__('*'),))
            x('abc 1\n', (ClassWith__str__('abc'), 1))
    
            # errors
            self.assertRaises(TypeError, print, '', sep=3)
            self.assertRaises(TypeError, print, '', end=3)
            self.assertRaises(AttributeError, print, '', file='')
    
        def test_print_flush(self):
            # operation of the flush flag
            class filelike:
                def __init__(self):
                    self.written = ''
                    self.flushed = 0
    
                def write(self, str):
                    self.written += str
    
                def flush(self):
                    self.flushed += 1
    
            f = filelike()
            print(1, file=f, end='', flush=True)
            print(2, file=f, end='', flush=True)
            print(3, file=f, flush=False)
            self.assertEqual(f.written, '123\n')
            self.assertEqual(f.flushed, 2)
    
            # ensure exceptions from flush are passed through
            class noflush:
                def write(self, str):
                    pass
    
                def flush(self):
                    raise RuntimeError
            self.assertRaises(RuntimeError, print, 1, file=noflush(), flush=True)
    
        def test_gh130163(self):
            class X:
                def __str__(self):
                    sys.stdout = StringIO()
                    support.gc_collect()
                    return 'foo'
    
            with support.swap_attr(sys, 'stdout', None):
                sys.stdout = StringIO()  # the only reference
                print(X())  # should not crash
    
    
print("read")
