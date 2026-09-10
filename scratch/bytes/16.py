print((-129).to_bytes(2, "big", signed=True), int.from_bytes(b"\x7f\xff", "little", signed=True))
print((128).to_bytes(2, "little", signed=True), (-1).to_bytes(3, "little", signed=True))
print("é".encode("utf-8", "strict"), b"\xc3\xa9".decode("utf8", "strict"), bytes("é", "utf-8", "strict"))
print(list(b"ABC"))
for n in b"AB":
    print(n)
