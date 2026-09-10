f = lambda x = lambda y = lambda z=1: z: y(): x()
print(f())
print((lambda a, *args, b, **kwds,: a + b)(1, 3, b=4, c=6))
if False:
    with (manager() as (x, y), manager() as z,):
        pass
print("read")
