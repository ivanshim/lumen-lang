print("a\rb\nc\r\nd\v".splitlines(keepends=True))
print("no".rpartition("!"), "a=b=c".rpartition("="), "é\tx\n\ty".expandtabs(tabsize=4))
print(" a  b  ".split(maxsplit=0), " a  b  ".rsplit(maxsplit=1), "--ab--".strip("-"))
print("aXbXc".rsplit(sep="X", maxsplit=1), "x".ljust(3, "é"), "+5".zfill(4))
