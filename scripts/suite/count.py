import sys, re, pathlib
def read(d):
    out = {}
    for f in sorted(pathlib.Path(d).glob("*.txt")):
        stem, k = f.name.split(".")[0], f.name.split(".")[1]
        t = f.read_text()
        prog = [l for l in t.splitlines() if l and set(l) <= set(".FEsx")]
        ran = re.search(r"^Ran (\d+) test", t, re.M)
        line = prog[0] if prog else ""
        if ran and len(line) != int(ran.group(1)): line = "?" + line
        out[(stem, k)] = (line, int(ran.group(1)) if ran else 0, "TIMEOUT" in t)
    return out
new, old = read(sys.argv[1]), read(sys.argv[2]) if len(sys.argv) > 2 else {}
for k in ("stack8", "microcode7"):
    p = r = files = nothing = 0
    for (stem, kk), (line, ran, to) in new.items():
        if kk != k: continue
        files += 1
        if not line or line.startswith("?"): nothing += 1; continue
        p += line.count("."); r += ran
        o = old.get((stem, kk))
        if o and o[0] and not o[0].startswith("?"):
            ol = o[0]
            if len(ol) == len(line):
                worse = [(i, ol[i], line[i]) for i in range(len(line)) if ol[i] == "." and line[i] in "EF"]
                if worse: print("  REGRESSION", stem, k, worse[:5])
            else:
                print("  length changed", stem, k, len(ol), "->", len(line))
            if line.count(".") != ol.count("."): print("  %-20s %-10s %d -> %d" % (stem, k, ol.count("."), line.count(".")))
        elif line: print("  NEW RUNNING", stem, k, line.count("."), "of", ran)
    print("%s: %d pass of %d ran, %d files, %d run nothing" % (k, p, r, files, nothing))
