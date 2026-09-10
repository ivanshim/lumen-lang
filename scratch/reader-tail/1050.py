# tests/python/test_grammar.py:1288
if False:
    def test_assert_failures(self):
        try:
            assert 0, "msg"
        except AssertionError as e:
            self.assertEqual(e.args[0], "msg")
        else:
            self.fail("AssertionError not raised by assert 0")

        try:
            assert False
        except AssertionError as e:
            self.assertEqual(len(e.args), 0)
        else:
            self.fail("AssertionError not raised by 'assert False'")


print('read')
