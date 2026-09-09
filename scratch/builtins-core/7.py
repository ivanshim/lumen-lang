for x in map(lambda v: v + 1, [1, 2]):
    print(x)
it = iter("xy")
alias = it
n = next
print(n(alias), n(it), n(alias, "end"))
f = sorted
print(f([2, 1], reverse=True))
print(*iter([3, 4]))
