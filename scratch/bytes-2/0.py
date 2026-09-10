ba = bytearray(b"ab"); ba[0] = 65; ba.append(0x43); print(ba, bytes(ba), ba == b"AbC", ba.upper())
