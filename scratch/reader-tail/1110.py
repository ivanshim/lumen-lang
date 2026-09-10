# tests/python/test_set.py:275
if False:
    def test_deepcopy(self):
        class Tracer:
            def __init__(self, value):
                self.value = value
            def __hash__(self):
                return self.value
            def __deepcopy__(self, memo=None):
                return Tracer(self.value + 1)
        t = Tracer(10)
        s = self.thetype([t])
        dup = copy.deepcopy(s)
        self.assertNotEqual(id(s), id(dup))
        for elem in dup:
            newt = elem
        self.assertNotEqual(id(t), id(newt))
        self.assertEqual(t.value + 1, newt.value)


print('read')
