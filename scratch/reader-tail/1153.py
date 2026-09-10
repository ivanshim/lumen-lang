# tests/python/test_set.py:796
if False:
    def test_hash_effectiveness(self):
        n = 13
        hashvalues = set()
        addhashvalue = hashvalues.add
        elemmasks = [(i+1, 1<<i) for i in range(n)]
        for i in range(2**n):
            addhashvalue(hash(frozenset([e for e, m in elemmasks if m&i])))
        self.assertEqual(len(hashvalues), 2**n)

        def zf_range(n):
            # https://en.wikipedia.org/wiki/Set-theoretic_definition_of_natural_numbers
            nums = [frozenset()]
            for i in range(n-1):
                num = frozenset(nums)
                nums.append(num)
            return nums[:n]

        def powerset(s):
            for i in range(len(s)+1):
                yield from map(frozenset, itertools.combinations(s, i))

        for n in range(18):
            t = 2 ** n
            mask = t - 1
            for nums in (range, zf_range):
                u = len({h & mask for h in map(hash, powerset(nums(n)))})
                self.assertGreater(4*u, t)


print('read')
