# tests/python/test_set.py:1944
if False:
    def make_sets_of_bad_objects(self):
        class Bad:
            def __eq__(self, other):
                if not enabled:
                    return False
                if randrange(20) == 0:
                    set1.clear()
                if randrange(20) == 0:
                    set2.clear()
                return bool(randrange(2))
            def __hash__(self):
                return randrange(2)
        # Don't behave poorly during construction.
        enabled = False
        set1 = self.constructor1(Bad() for _ in range(randrange(50)))
        set2 = self.constructor2(Bad() for _ in range(randrange(50)))
        # Now start behaving poorly
        enabled = True
        return set1, set2


print('read')
