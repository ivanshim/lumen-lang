# tests/python/test_grammar.py:1524
if False:
    def test_warn_missed_comma(self):
        def check(test):
            self.check_syntax_warning(test, msg)

        msg=r'is not callable; perhaps you missed a comma\?'
        check('[(1, 2) (3, 4)]')
        check('[(x, y) (3, 4)]')
        check('[[1, 2] (3, 4)]')
        check('[{1, 2} (3, 4)]')
        check('[{1: 2} (3, 4)]')
        check('[[i for i in range(5)] (3, 4)]')
        check('[{i for i in range(5)} (3, 4)]')
        check('[(i for i in range(5)) (3, 4)]')
        check('[{i: i for i in range(5)} (3, 4)]')
        check('[f"{x}" (3, 4)]')
        check('[f"x={x}" (3, 4)]')
        check('["abc" (3, 4)]')
        check('[b"abc" (3, 4)]')
        check('[123 (3, 4)]')
        check('[12.3 (3, 4)]')
        check('[12.3j (3, 4)]')
        check('[None (3, 4)]')
        check('[True (3, 4)]')
        check('[... (3, 4)]')
        check('[t"{x}" (3, 4)]')
        check('[t"x={x}" (3, 4)]')

        msg=r'is not subscriptable; perhaps you missed a comma\?'
        check('[{1, 2} [i, j]]')
        check('[{i for i in range(5)} [i, j]]')
        check('[(i for i in range(5)) [i, j]]')
        check('[(lambda x, y: x) [i, j]]')
        check('[123 [i, j]]')
        check('[12.3 [i, j]]')
        check('[12.3j [i, j]]')
        check('[None [i, j]]')
        check('[True [i, j]]')
        check('[... [i, j]]')
        check('[t"{x}" [i, j]]')
        check('[t"x={x}" [i, j]]')

        msg=r'indices must be integers or slices, not tuple; perhaps you missed a comma\?'
        check('[(1, 2) [i, j]]')
        check('[(x, y) [i, j]]')
        check('[[1, 2] [i, j]]')
        check('[[i for i in range(5)] [i, j]]')
        check('[f"{x}" [i, j]]')
        check('[f"x={x}" [i, j]]')
        check('["abc" [i, j]]')
        check('[b"abc" [i, j]]')

        msg=r'indices must be integers or slices, not tuple;'
        check('[[1, 2] [3, 4]]')
        msg=r'indices must be integers or slices, not list;'
        check('[[1, 2] [[3, 4]]]')
        check('[[1, 2] [[i for i in range(5)]]]')
        msg=r'indices must be integers or slices, not set;'
        check('[[1, 2] [{3, 4}]]')
        check('[[1, 2] [{i for i in range(5)}]]')
        msg=r'indices must be integers or slices, not dict;'
        check('[[1, 2] [{3: 4}]]')
        check('[[1, 2] [{i: i for i in range(5)}]]')
        msg=r'indices must be integers or slices, not generator;'
        check('[[1, 2] [(i for i in range(5))]]')
        msg=r'indices must be integers or slices, not function;'
        check('[[1, 2] [(lambda x, y: x)]]')
        msg=r'indices must be integers or slices, not str;'
        check('[[1, 2] [f"{x}"]]')
        check('[[1, 2] [f"x={x}"]]')
        check('[[1, 2] ["abc"]]')
        msg=r'indices must be integers or slices, not string.templatelib.Template;'
        check('[[1, 2] [t"{x}"]]')
        check('[[1, 2] [t"x={x}"]]')
        msg=r'indices must be integers or slices, not'
        check('[[1, 2] [b"abc"]]')
        check('[[1, 2] [12.3]]')
        check('[[1, 2] [12.3j]]')
        check('[[1, 2] [None]]')
        check('[[1, 2] [...]]')

        with warnings.catch_warnings():
            warnings.simplefilter('error', SyntaxWarning)
            compile('[(lambda x, y: x) (3, 4)]', '<testcase>', 'exec')
            compile('[[1, 2] [i]]', '<testcase>', 'exec')
            compile('[[1, 2] [0]]', '<testcase>', 'exec')
            compile('[[1, 2] [True]]', '<testcase>', 'exec')
            compile('[[1, 2] [1:2]]', '<testcase>', 'exec')
            compile('[{(1, 2): 3} [i, j]]', '<testcase>', 'exec')


print('read')
