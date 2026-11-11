# The module alias was bound to the Python method above. Point both the
# class and that alias at the native descriptor, then drop the installer
# so it is not part of the _random surface after this module loads.
_randrange_python = Random.randrange
Random.randrange = _random._bind_randrange(_randrange_python, Random._randbelow_with_getrandbits)
randrange = _inst.randrange
del _randrange_python
del _random._bind_randrange
