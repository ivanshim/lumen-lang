# tests/python/test_set.py:1630
if False:
        def setUp(self):
            def gen():
                for i in range(0, 10, 2):
                    yield i
            self.set   = set((1, 2, 3))
            self.other = gen()
            self.otherIsIterable = True

    #==============================================================================


print('read')
