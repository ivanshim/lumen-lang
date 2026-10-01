# Structural validator adapted from CPython 3b564385e4c9 Modules/_sre/sre.c; PSF License.

def validate(code, groups):
    """Reject bytecode outside the compiler's structured SRE instruction format."""
    size = len(code)
    def bad():
        raise RuntimeError('invalid SRE code')
    def arg(at, end):
        if at < 0 or at >= end:
            bad()
        return code[at]
    def jump(at, end, adjustment=0):
        skip = arg(at, end)
        if skip < adjustment or skip - adjustment > end - at:
            bad()
        return skip
    def category(value):
        if value >= 68:
            bad()
    def charset(at, end):
        if at < 0 or at > end:
            bad()
        while at < end:
            op = code[at]
            at += 1
            if op == 21:  # NEGATE
                continue
            if op == 16:  # LITERAL
                arg(at, end)
                at += 1
            elif op in (22, 42):  # RANGE, RANGE_UNI_IGNORE
                arg(at + 1, end)
                at += 2
            elif op == 9:  # CHARSET: eight 32-bit words
                at += 8
            elif op == 10:  # BIGCHARSET
                blocks = arg(at, end)
                at += 1
                if at + 64 > end:
                    bad()
                for word in code[at:at + 64]:
                    for shift in (0, 8, 16, 24):
                        if (word >> shift) & 255 >= blocks:
                            bad()
                at += 64 + blocks * 8
            elif op == 8:
                category(arg(at, end))
                at += 1
            else:
                bad()
            if at > end:
                bad()
    def inner(at, end):
        if at < 0 or end > size or at > end:
            bad()
        while at < end:
            op = code[at]
            at += 1
            if op == 17:  # MARK
                if arg(at, end) >= 2 * groups:
                    bad()
                at += 1
            elif op in (16, 20, 32, 33, 36, 37, 40, 41):
                arg(at, end)
                at += 1
            elif op in (0, 1, 2, 3):
                pass
            elif op == 6:  # AT
                if arg(at, end) >= 12:
                    bad()
                at += 1
            elif op == 8:
                category(arg(at, end))
                at += 1
            elif op in (13, 31, 35, 39):  # IN families
                skip = jump(at, end)
                start = at + 1
                target = at + skip
                charset(start, target - 1)
                if arg(target - 1, end) != 0:
                    bad()
                at = target
            elif op == 14:  # INFO
                skip = jump(at, end)
                target = at + skip
                at += 1
                flags = arg(at, end)
                arg(at + 2, end)
                at += 3
                if flags & ~7 or flags & 1 and flags & 4 or flags & 2 and not flags & 1:
                    bad()
                if flags & 1:
                    count = arg(at, end)
                    arg(at + 1, end)
                    at += 2
                    if count > target - at:
                        bad()
                    at += count
                    if count > target - at:
                        bad()
                    for overlap in code[at:at + count]:
                        if overlap >= count:
                            bad()
                    at += count
                if flags & 4:
                    charset(at, target - 1)
                    if arg(target - 1, end) != 0:
                        bad()
                    at = target
                elif at != target:
                    bad()
            elif op == 7:  # BRANCH
                target = None
                while True:
                    skip = jump(at, end)
                    at += 1
                    if skip == 0:
                        break
                    if inner(at, at + skip - 3):
                        bad()
                    at += skip - 3
                    if arg(at, end) != 15:
                        bad()
                    at += 1
                    skip = jump(at, end)
                    destination = at + skip
                    at += 1
                    if target is None:
                        target = destination
                    elif destination != target:
                        bad()
                if at != target:
                    bad()
            elif op in (24, 26, 29, 23, 28):  # repeats
                skip = jump(at, end)
                at += 1
                minimum = arg(at, end)
                maximum = arg(at + 1, end)
                at += 2
                if minimum > maximum or maximum > 4294967295:
                    bad()
                one = op in (24, 26, 29)
                stop = at + skip - (4 if one else 3)
                if inner(at, stop):
                    bad()
                terminal = arg(stop, end)
                if one or op == 28:
                    if terminal != 1:
                        bad()
                elif terminal not in (18, 19):
                    bad()
                at = stop + 1
            elif op == 27:  # ATOMIC_GROUP
                skip = jump(at, end)
                at += 1
                stop = at + skip - 2
                if inner(at, stop) or arg(stop, end) != 1:
                    bad()
                at = stop + 1
            elif op in (11, 30, 34, 38):
                if arg(at, end) >= groups:
                    bad()
                at += 1
            elif op == 12:  # GROUPREF_EXISTS
                if arg(at, end) >= groups:
                    bad()
                at += 1
                skip = jump(at, end, 1)
                rc = inner(at + 1, at + skip - 1)
                if rc == 1:
                    at += skip - 2
                    skip = jump(at, end)
                    at += 1
                    rc = inner(at, at + skip - 1)
                if rc:
                    bad()
                at += skip - 1
            elif op in (4, 5):
                skip = jump(at, end)
                at += 1
                arg(at, end)
                stop = at + skip - 2
                if inner(at + 1, stop) or arg(stop, end) != 1:
                    bad()
                at = stop + 1
            elif op == 15:
                if at + 1 != end:
                    bad()
                return 1
            else:
                bad()
        return 0
    if groups < 0 or groups > 1073741823 or not size or code[-1] != 1:
        bad()
    if inner(0, size - 1):
        bad()
