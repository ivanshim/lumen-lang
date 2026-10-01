# SHA-512 adapter; PSF License.
from _random_seed import digest as _digest
class sha512:
    def __init__(self, data=b'', *, usedforsecurity=True):
        self._data = bytes(data)
    def digest(self):
        return _digest(self._data)
    def hexdigest(self):
        return self.digest().hex()
    def update(self, data):
        self._data += bytes(data)
    def copy(self):
        return sha512(self._data)
