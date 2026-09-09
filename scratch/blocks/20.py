def decorate(f): return f
@decorate
async def f(): return 4
print(await f())
