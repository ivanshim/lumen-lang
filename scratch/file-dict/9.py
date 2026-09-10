options = {"sep": ":", "end": "!\n"}
print(1, 2, **options)
print(*[3, 4], **{"sep": "-"})
print(**{"end": "empty\n"})
print(5, 6, sep=None, end=None)
print(7, end="")
print(8)
