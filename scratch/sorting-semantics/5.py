words = ["b", "A", "a", "Z"]
words.sort(key=str.lower)
print(words)
print(sorted(["bbb", "a", "cc", "d"], key=len))
print(sorted(["a", "Z"]), sorted([True, 1.5, 0]))
print(sorted((3, 1, 2)), sorted({3, 1, 2}), sorted(x for x in range(3)))
print(list(reversed(sorted([3, 1, 2]))))
print(sorted({"b": 2, "a": 1}.items()))
print(sorted([(1, 2), (1, 1), (0, 9)]))
