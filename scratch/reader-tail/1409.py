# tests/python/test_with.py:668
if False:
    def testSingleComplexTarget(self):
        targets = {1: [0, 1, 2]}
        with mock_contextmanager_generator() as targets[1][0]:
            self.assertEqual(list(targets.keys()), [1])
            self.assertEqual(targets[1][0].__class__, MockResource)
        with mock_contextmanager_generator() as list(targets.values())[0][1]:
            self.assertEqual(list(targets.keys()), [1])
            self.assertEqual(targets[1][1].__class__, MockResource)
        with mock_contextmanager_generator() as targets[2]:
            keys = list(targets.keys())
            keys.sort()
            self.assertEqual(keys, [1, 2])
        class C: pass
        blah = C()
        with mock_contextmanager_generator() as blah.foo:
            self.assertHasAttr(blah, "foo")


print('read')
