def read(a):
    return a[:42, ..., :24:, 24, 100]

def write(a):
    a[:42, ..., :24:, 24, 100] = "Strange"

read([0, 1, 2])
