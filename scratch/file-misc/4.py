async def f(a=1, /, *, b=2):
    async with manager as result:
        await result
