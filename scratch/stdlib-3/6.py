import heapq, statistics, string, math, operator
print(heapq.nsmallest(3, [3, 1, 2, 1]), heapq.nlargest(2, [1, 9, 4]))
print(list(heapq.merge([2, 1], [3])))
print(statistics.mode([2, 1, 2, 1]), statistics.variance([1, 2, 3]), statistics.stdev([1, 2, 3]))
print(string.Formatter().format('{} {name}', 7, name='ok'))
print(math.remainder(7, 2), math.remainder(5, 2), operator.iadd(2, 3))
print(math.ldexp(1.0, -1074) > 0, math.ldexp(1.0, -1075) == 0, math.ldexp(math.inf, -1000000) == math.inf)
