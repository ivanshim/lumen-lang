# tests/python/test_set.py:1401
if False:
        def test_issubset(self):
            x = self.left
            y = self.right
            for case in "!=", "==", "<", "<=", ">", ">=":
                expected = case in self.cases
                # Test the binary infix spelling.
                result = eval("x" + case + "y", locals())
                self.assertEqual(result, expected)
                # Test the "friendly" method-name spelling, if one exists.
                if case in TestSubsets.case2method:
                    method = getattr(x, TestSubsets.case2method[case])
                    result = method(y)
                    self.assertEqual(result, expected)

                # Now do the same for the operands reversed.
                rcase = TestSubsets.reverse[case]
                result = eval("y" + rcase + "x", locals())
                self.assertEqual(result, expected)
                if rcase in TestSubsets.case2method:
                    method = getattr(y, TestSubsets.case2method[rcase])
                    result = method(x)
                    self.assertEqual(result, expected)
    #------------------------------------------------------------------------------


print('read')
