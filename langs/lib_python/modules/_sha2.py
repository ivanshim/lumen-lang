# Interface for CPython v3.14.8 Modules/sha2module.c.
from _digest import Hash, Shake, Blake

class sha224(Hash):
    _algorithm = 'sha224'
    _digest_size = 28
    _block_size = 64

class sha256(Hash):
    _algorithm = 'sha256'
    _digest_size = 32
    _block_size = 64

class sha384(Hash):
    _algorithm = 'sha384'
    _digest_size = 48
    _block_size = 128

class sha512(Hash):
    _algorithm = 'sha512'
    _digest_size = 64
    _block_size = 128


__crypto(4, sha224)
__crypto(4, sha256)
__crypto(4, sha384)
__crypto(4, sha512)
