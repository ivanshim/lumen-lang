# tests/python/test_set.py:1142
if False:
        def test_constructor(self):
            inner = frozenset([1])
            outer = set([inner])
            element = outer.pop()
            self.assertEqual(type(element), frozenset)
            outer.add(inner)        # Rebuild set of sets with .add method
            outer.remove(inner)
            self.assertEqual(outer, set())   # Verify that remove worked
            outer.discard(inner)    # Absence of KeyError indicates working fine

    #==============================================================================


print('read')
