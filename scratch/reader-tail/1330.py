# tests/python/test_set.py:2021
if False:
    def test_ixor_with_mutation(self):
        def f(a, b):
            a ^= b
        self.check_set_op_does_not_crash(f)


print('read')
