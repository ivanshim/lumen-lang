# Interface for CPython v3.14.8 Modules/md5module.c.
from _digest import Hash, Shake, Blake

class md5(Hash):
    _algorithm = 'md5'
    _digest_size = 16
    _block_size = 64


__crypto(4, md5)
