# Interface for CPython v3.14.8 Modules/sha3module.c.
from _digest import Hash, Shake, Blake

class sha3_224(Hash):
    _algorithm = 'sha3_224'
    _digest_size = 28
    _block_size = 144
    _rate_bits = 1152
    _capacity_bits = 448
    _suffix = b'\x06'

class sha3_256(Hash):
    _algorithm = 'sha3_256'
    _digest_size = 32
    _block_size = 136
    _rate_bits = 1088
    _capacity_bits = 512
    _suffix = b'\x06'

class sha3_384(Hash):
    _algorithm = 'sha3_384'
    _digest_size = 48
    _block_size = 104
    _rate_bits = 832
    _capacity_bits = 768
    _suffix = b'\x06'

class sha3_512(Hash):
    _algorithm = 'sha3_512'
    _digest_size = 64
    _block_size = 72
    _rate_bits = 576
    _capacity_bits = 1024
    _suffix = b'\x06'

class shake_128(Shake):
    _algorithm = 'shake_128'
    _digest_size = 0
    _block_size = 168
    _rate_bits = 1344
    _capacity_bits = 256
    _suffix = b'\x1f'

class shake_256(Shake):
    _algorithm = 'shake_256'
    _digest_size = 0
    _block_size = 136
    _rate_bits = 1088
    _capacity_bits = 512
    _suffix = b'\x1f'


__crypto(4, sha3_224)
__crypto(4, sha3_256)
__crypto(4, sha3_384)
__crypto(4, sha3_512)
__crypto(4, shake_128)
__crypto(4, shake_256)
