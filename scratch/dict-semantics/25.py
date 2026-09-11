d = {}
try:
    d.update([('a', 1), ('wrong',)])
except:
    print(d)
e = {}
try:
    e |= [('b', 2), ('wrong',)]
except:
    print(e)
