def plain(f):
    return f

def maker(*args, **kwds):
    print(args[0])
    print(kwds['size'])
    return plain

@maker(1, size=42)
@maker(*[2], **{'size': 84})
@maker(3, size=126,)
def answer():
    return 42

print(answer())
