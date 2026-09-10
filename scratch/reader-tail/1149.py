# tests/python/test_set.py:765
if False:
    def test_hash(self):
        self.assertEqual(hash(self.thetype('abcdeb')),
                         hash(self.thetype('ebecda')))

        # make sure that all permutations give the same hash value
        n = 100
        seq = [randrange(n) for i in range(n)]
        results = set()
        for i in range(200):
            shuffle(seq)
            results.add(hash(self.thetype(seq)))
        self.assertEqual(len(results), 1)


print('read')
