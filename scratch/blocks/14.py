with 5 as (x):
    print(x)
with [1, [2, 3]] as (a, (b, c)):
    print(a, b, c)
async for x, y in [[1, 2], [3, 4]]:
    print(x, y)
else:
    print("done")
