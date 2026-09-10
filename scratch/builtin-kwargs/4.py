import sys
stream = sys.stderr
print("error", 7, sep=":", end="!\n", file=stream, flush=True)
print(sep=None, end=None, file=None)
print(sep="/", *[1, 2], **{"end": "!\n"})
