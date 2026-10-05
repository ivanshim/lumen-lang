# Interface for CPython v3.14.8 Modules/blake2module.c.
from _digest import Hash, Shake, Blake

class blake2b(Blake):
    _algorithm = 'blake2b'
    _digest_size = 64
    _block_size = 128
    MAX_DIGEST_SIZE = 64
    MAX_KEY_SIZE = 64
    SALT_SIZE = 16
    PERSON_SIZE = 16
    _max_offset = 18446744073709551615

class blake2s(Blake):
    _algorithm = 'blake2s'
    _digest_size = 32
    _block_size = 64
    MAX_DIGEST_SIZE = 32
    MAX_KEY_SIZE = 32
    SALT_SIZE = 8
    PERSON_SIZE = 8
    _max_offset = 281474976710655

BLAKE2B_SALT_SIZE = 16
BLAKE2B_PERSON_SIZE = 16
BLAKE2B_MAX_KEY_SIZE = 64
BLAKE2B_MAX_DIGEST_SIZE = 64
BLAKE2S_SALT_SIZE = 8
BLAKE2S_PERSON_SIZE = 8
BLAKE2S_MAX_KEY_SIZE = 32
BLAKE2S_MAX_DIGEST_SIZE = 32

__crypto(4, blake2b)
__crypto(4, blake2s)
