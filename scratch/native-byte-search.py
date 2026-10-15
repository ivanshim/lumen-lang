state=17
def draw(n):
    global state
    state=(state*1103515245+12345)%2147483648
    return state%n
def positions(hay,needle):
    return [i for i in range(len(hay)-len(needle)+1) if hay[i:i+len(needle)]==needle]
for kind in (bytes,bytearray):
    for i in range(300):
        hay=bytes([draw(3)*127 for j in range(draw(70))])
        needle=bytes([draw(3)*127 for j in range(draw(18))])
        if i%3==0:
            needle=hay[draw(len(hay)+1):]
        h=kind(hay);p=kind(needle);found=positions(hay,needle)
        assert h.find(p)==(found[0] if found else -1)
        assert h.rfind(p)==(found[-1] if found else -1)
        count=0;next_at=0
        for at in found:
            if at>=next_at:
                count+=1;next_at=at+max(1,len(needle))
        assert h.count(p)==count
        if found:
            assert h.index(p)==found[0]
            assert h.rindex(p)==found[-1]
        else:
            for method in (h.index,h.rindex):
                try: method(p)
                except ValueError: pass
                else: raise AssertionError('missing byte-search error')
    long=b'\xff'*12+b'\x00'
    hay=kind(b'\xff'*80+b'\x00'+b'\xff'*12+b'\x00')
    assert hay.find(long)==68
    assert hay.rfind(long)==81
    assert hay.count(long)==2
    assert hay.find(long,69)==81
    assert hay.rfind(long,0,81)==68
    assert hay.split(long)==[kind(b'\xff'*68),kind(b''),kind(b'')]
    assert hay.rsplit(long,1)==[kind(bytes(hay)[:81]),kind(b'')]
    repeated=kind(b'a'*30)
    assert repeated.find(b'a'*9)==0
    assert repeated.rfind(b'a'*9)==21
    assert repeated.count(b'a'*9)==3
print('byte searches preserve binary data, overlap, bounds, errors and splits')
