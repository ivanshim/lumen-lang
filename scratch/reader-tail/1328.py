# tests/python/test_set.py:2011
if False:
    def test_ior_with_mutation(self):
        def f(a, b):
            a |= b
        self.check_set_op_does_not_crash(f)


print('read')
