def dormant(sequences):
    for left, right in ('ba', '\u0101\u0100'):
        print(left, right)
    for (x, y) in sequences:
        print(x, y)
    for n, (seq, res) in enumerate(sequences):
        print(n, seq, res)
def dormant_return(sequences):
    for x, y in sequences:
        if x:
            continue
        if y:
            break
        return x
    return 42
print("read")
