def dormant(sequences):
    for left, right in ('ba', '\u0101\u0100'):
        print(left, right)
    for (x, y) in sequences:
        print(x, y)
    for n, (seq, res) in enumerate(sequences):
        print(n, seq, res)
print("read")
