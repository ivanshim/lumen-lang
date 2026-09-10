# tests/python/test_set.py:1910
if False:
    def test_hash_collision_concurrent_add(self):
        class X:
            def __hash__(self):
                return 0
        class Y:
            flag = False
            def __hash__(self):
                return 0
            def __eq__(self, other):
                if not self.flag:
                    self.flag = True
                    s.add(X())
                return self is other

        a = X()
        s = set()
        s.add(a)
        s.add(X())
        s.remove(a)
        # Now the set contains a dummy entry followed by an entry
        # for an object with hash 0.
        s.add(Y())
        # The following operations should not crash.
        repr(s)
        list(s)
        set() | s



print('read')
