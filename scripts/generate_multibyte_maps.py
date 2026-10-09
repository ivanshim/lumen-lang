# Generate CJK data with CPython 3.12+; mapping headers match v3.14.8.
# See langs/lib_python/data/multibyte/README.md for provenance and binary layout.
from pathlib import Path
import codecs, struct
root=Path('langs/lib_python/data/multibyte');root.mkdir(parents=True,exist_ok=True)
names='big5 big5hkscs cp932 cp949 cp950 euc_jp euc_jis_2004 euc_jisx0213 euc_kr gb2312 gbk gb18030 johab shift_jis shift_jis_2004 shift_jisx0213'.split()
for name in names:
    enc={};dec={};ranges=[];previous=None
    for cp in range(0x110000):
        try:b=chr(cp).encode(name)
        except UnicodeEncodeError:continue
        if len(b)==4 and name=='gb18030':
            a,c,d,e=b;pointer=((a-129)*10+c-48)*1260+(d-129)*10+e-48
            if previous and previous[1]+1==cp and previous[3]+1==pointer:previous=(previous[0],cp,previous[2],pointer);ranges[-1]=previous
            else:previous=(cp,cp,pointer,pointer);ranges.append(previous)
        else:enc[(cp,0)]=b
    units=[bytes([v]) for v in range(256)]
    units.extend(bytes([a,b]) for a in range(128,256) for b in range(256))
    if name.startswith('euc_j'):
        units.extend(bytes([143,a,b]) for a in range(161,255) for b in range(161,255))
    for b in units:
        try:text=b.decode(name)
        except UnicodeDecodeError:continue
        if not 0<len(text)<=2:continue
        # Only units consumed atomically; two ASCII units are not a double-byte character.
        if len(text)==2 and not(name in ('big5hkscs','euc_jis_2004','euc_jisx0213','shift_jis_2004','shift_jisx0213') and ord(text[0])>127):continue
        points=tuple(map(ord,text));dec[(len(b),int.from_bytes(b,'big'))]=(points[0],points[1] if len(points)==2 else 0)
        if len(points)==2:
            encoded=text.encode(name)
            if len(encoded)<=3:enc[points]=encoded
    for points, b in enc.items():
        if len(b)>3:
            dec[(len(b),int.from_bytes(b,'big'))]=points
    # Fixed records let each kernel search independently without interpreting enormous dict literals.
    data=struct.pack('<II',len(enc),len(dec))
    for (a,b),v in sorted(enc.items()):data+=struct.pack('<IIIQ',a,b,len(v),int.from_bytes(v,'big'))
    for (length,raw),(a,b) in sorted(dec.items()):data+=struct.pack('<IQII',length,raw,a,b)
    data+=struct.pack('<I',len(ranges))
    for a,b,c,d in ranges:data+=struct.pack('<IIII',a,b,c,d)
    (root/(name+'.bin')).write_bytes(data)
    print(name,len(enc),len(dec),len(ranges),len(data),flush=True)

for name in ('iso2022_jp','iso2022_jp_1','iso2022_jp_2','iso2022_jp_2004','iso2022_jp_3','iso2022_jp_ext','iso2022_kr','hz'):
    enc={};dec={}
    for cp in range(0x110000):
        try:b=chr(cp).encode(name)
        except UnicodeEncodeError:continue
        if b.endswith(b'\x1b(B'):b=b[:-3]
        elif name=='iso2022_kr' and b.endswith(b'\x0f'):b=b[:-1]
        elif name=='hz' and b.endswith(b'~}'):b=b[:-2]
        assert len(b)<=8,(name,cp,b)
        enc[(cp,0)]=b
        try:text=b.decode(name)
        except UnicodeDecodeError:continue
        points=tuple(map(ord,text))
        if len(points)!=1:continue
        dec[(len(b),int.from_bytes(b,'big'))]=(points[0],0)
    # JIS X 0213 two-character mappings.
    if name in ('iso2022_jp_2004','iso2022_jp_3'):
        for a in range(33,127):
            for b in range(33,127):
                wire=(b'\x1b$(Q' if name.endswith('2004') else b'\x1b$(O')+bytes([a,b])
                try:text=wire.decode(name)
                except UnicodeDecodeError:continue
                if len(text)==2:
                    points=tuple(map(ord,text));enc[points]=wire;dec[(len(wire),int.from_bytes(wire,'big'))]=points
    if name == 'hz':
        prefixes = [(b'~{',2)]
    elif name == 'iso2022_kr':
        prefixes = [(b'\x1b$)C\x0e',2)]
    else:
        prefixes = [(b'\x1b('+bytes([mark]),1) for mark in (66,73,74)]
        prefixes += [(b'\x1b$'+bytes([mark]),2) for mark in (64,65,66)]
        prefixes += [(b'\x1b$('+bytes([mark]),2) for mark in (67,68,79,80,81)]
        prefixes += [(b'\x1b.'+bytes([mark])+b'\x1bN',1) for mark in (65,70)]
    for prefix,width in prefixes:
        for first in range(33,127):
            for second in (range(33,127) if width == 2 else [None]):
                wire = prefix+bytes([first] if second is None else [first,second])
                try:text=wire.decode(name)
                except UnicodeDecodeError:continue
                if 0<len(text)<=2:
                    points=tuple(map(ord,text))
                    dec[(len(wire),int.from_bytes(wire,'big'))]=(points[0],points[1] if len(points)==2 else 0)
    out=bytearray(struct.pack('<II',len(enc),len(dec)))
    for (a,b),wire in sorted(enc.items()):out.extend(struct.pack('<IIIQ',a,b,len(wire),int.from_bytes(wire,'big')))
    for (length,raw),(a,b) in sorted(dec.items()):out.extend(struct.pack('<IQII',length,raw,a,b))
    out.extend(struct.pack('<I',0));(root/(name+'.bin')).write_bytes(out)
    print(name,len(enc),len(out),flush=True)
