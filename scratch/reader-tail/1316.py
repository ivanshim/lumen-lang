# tests/python/test_set.py:1964
if False:
    def check_set_op_does_not_crash(self, function):
        for _ in range(100):
            set1, set2 = self.make_sets_of_bad_objects()
            try:
                function(set1, set2)
            except RuntimeError as e:
                # Just make sure we don't crash here.
                self.assertIn("changed size during iteration", str(e))



print('read')
