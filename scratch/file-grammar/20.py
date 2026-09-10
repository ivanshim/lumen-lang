def contexts():
    with manager():
        pass
    with manager() as x, manager() as (y, z):
        pass
    with (
        manager() as (x, [y, z]),
        manager(),
    ):
        return 1
print('read')
