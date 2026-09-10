import bisect, heapq
a = [1, 3, 5]
bisect.insort(a, 4)
h = [5, 1, 3]
heapq.heapify(h)
print(a, bisect.bisect_left(a, 3), heapq.heappop(h), heapq.nlargest(2, [1, 9, 4]))
