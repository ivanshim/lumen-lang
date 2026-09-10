# tests/python/test_set.py:2026
if False:
    def test_iteration_with_mutation(self):
        def f1(a, b):
            for x in a:
                pass
            for y in b:
                pass
        def f2(a, b):
            for y in b:
                pass
            for x in a:
                pass
        def f3(a, b):
            for x, y in zip(a, b):
                pass
        self.check_set_op_does_not_crash(f1)
        self.check_set_op_does_not_crash(f2)
        self.check_set_op_does_not_crash(f3)



print('read')
