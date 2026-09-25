import sys, re, pathlib
# usage: progcmp.py <fixcheck.log> <repo>  -- compares each program's new progress line to the fixture's
root = pathlib.Path(sys.argv[2]); log = open(sys.argv[1]).read()
blocks = re.findall(r'^(\S+) +(stack8|microcode7) +exit=(-?\d+) +(\S+).*\n    first=(.*)\n    prog=(.*)$', log, re.M)
by = {}
for prog, k, st, ok, first, line in blocks: by.setdefault(prog, {})[k] = (st, ok, first, line)
for prog, ks in by.items():
    if len(ks) < 2: print(prog, "INCOMPLETE"); continue
    a, b = ks['stack8'], ks['microcode7']
    agree = a[3] == b[3] and a[2][:100] == b[2][:100]
    err = root/"scratch"/(prog + ".err")
    old = err.read_text().splitlines()[0] if err.exists() else ""
    new = a[3]
    print("%-18s agree=%s stack8=%s micro=%s" % (prog, agree, a[1], b[1]))
    if old and new and set(old) <= set(".FEsx"):
        n = min(len(old), len(new))
        changes = [(i, old[i], new[i]) for i in range(n) if old[i] != new[i]]
        worse = [c for c in changes if c[1] == '.' and c[2] in 'EF']
        print("   len %d -> %d, %d changes, %d dot->E/F: %s" % (len(old), len(new), len(changes), len(worse), worse[:8]))
        if len(old) != len(new): print("   LENGTH DIFFERS, shift check needed")
    if not agree:
        print("   stack8:", a[3] or a[2][:100]); print("   micro :", b[3] or b[2][:100])
