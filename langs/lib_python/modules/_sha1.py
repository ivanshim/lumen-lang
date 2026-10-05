# Interface for CPython v3.14.8 Modules/sha1module.c.
from _digest import Hash, Shake, Blake

class sha1(Hash):
    _algorithm = 'sha1'
    _digest_size = 20
    _block_size = 64


__crypto(4, sha1)
