k = 99
actual = {k: k + 10 for k in range(10)}
print(len(actual), actual[0], actual[9], k)
actual = {k: v for k in range(10) for v in range(10) if k == v}
print(len(actual), actual[9])
print(sum({j * j for i in range(4) for j in [i + 1]}))
