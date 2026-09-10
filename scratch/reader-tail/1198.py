# tests/python/test_set.py:1129
if False:
        def test_changingSizeWhileIterating(self):
            s = set([1,2,3])
            try:
                for i in s:
                    s.update([4])
            except RuntimeError:
                pass
            else:
                self.fail("no exception when changing size during iteration")

    #==============================================================================


print('read')
