print(b"ab".hex(), bytes.fromhex("6162"), b"AB".lower(), b"a,b".split(b","), (258).to_bytes(2, "big"), int.from_bytes(b"\x01\x02", "little"))
