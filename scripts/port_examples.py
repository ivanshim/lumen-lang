#!/usr/bin/env python3
"""Port the Lumen example suite to every other language definition.

Reads examples/lumen/**/*.lm and the library functions they use from
lib_lumen/*.lm, parses them, and writes each example in the spelling of
every other definition in langs/ and langs/extras/, mirroring the Lumen
directory layout: examples/lumen/constructs/x.lm becomes
examples/python/constructs/x.py. The library functions an example calls
are ported along with it, so the output is self-contained.

An example is written for a language only when the language's definition
spells every construct the example (and its library functions) needs; the
first missing spelling is the reason recorded in examples/PORTS.md.
Constructs the target lacks a keyword for but can express otherwise are
rewritten: `until c` as `while not c`, a range loop as a while loop, the
pipe as a nested call, base-N literals as decimal, and a function's tail
expression as an explicit return. Builtins are called the way the language
writes them (METHOD_FORM: `arr.push(x)`, `s.length`), a library constant a
function reads is inlined where functions cannot see top-level names, and
Pascal gets its var sections and result-by-name functions.

Ports are spelled as the definition says the language is spelled; they are
run by the kernels, not compiled by the languages' own toolchains.
"""
import json
import re
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LUMEN_EXAMPLES = ROOT / "examples" / "lumen"
LIB = ROOT / "lib_lumen"
LANGS = ROOT / "langs"
REPORT = ROOT / "examples" / "PORTS.md"

HEADER = "Ported from {source} by scripts/port_examples.py; edit the Lumen original, not this file."


class Skip(Exception):
    """The target language has no spelling for something the example needs."""


# ---------------------------------------------------------------- lexing

KEYWORDS = {
    "let", "mut", "fn", "if", "else", "while", "until", "for", "in", "return",
    "break", "continue", "and", "or", "not", "true", "false", "null", "extern",
}
OPERATORS = ["|>", "..", "**", "//", "==", "!=", "<=", ">=", "+", "-", "*", "/", "%", ".",
             "<", ">", "=", "(", ")", "[", "]", ",", ":", "!"]


class Tok:
    def __init__(self, kind, value, line):
        self.kind, self.value, self.line = kind, value, line

    def __repr__(self):
        return f"{self.kind}:{self.value!r}"


def unterminated_string(line):
    """Whether the line ends inside a string literal."""
    quote = None
    i = 0
    while i < len(line):
        c = line[i]
        if quote is None:
            if c == "#":
                return False
            if c in "\"'":
                quote = c
        elif c == "\\":
            i += 1
        elif c == quote:
            quote = None
        i += 1
    return quote is not None


def lex(source):
    """Tokens with INDENT/DEDENT/NEWLINE from Lumen's four-space indentation."""
    toks = []
    indents = [0]
    depth = 0  # bracket depth suspends line structure
    lines = source.split("\n")
    # A string may hold a literal line break: join such lines first.
    joined = []
    buf = None
    for raw in lines:
        text = raw if buf is None else buf + "\n" + raw
        if unterminated_string(text):
            buf = text
            continue
        joined.append(text)
        buf = None
    if buf is not None:
        joined.append(buf)
    for lineno, raw in enumerate(joined, 1):
        line = raw.rstrip()
        i = 0
        spaces = len(line) - len(line.lstrip(" "))
        stripped = line.strip()
        if depth == 0:
            if not stripped or stripped.startswith("#"):
                continue
            if spaces > indents[-1]:
                indents.append(spaces)
                toks.append(Tok("INDENT", None, lineno))
            while spaces < indents[-1]:
                indents.pop()
                toks.append(Tok("DEDENT", None, lineno))
        i = spaces
        while i < len(line):
            c = line[i]
            if c == " " or c == "\t":
                i += 1
                continue
            if c == "#":
                break
            if c.isdigit():
                m = re.match(r"\d+@[0-9A-Za-z]+(?:\.[0-9A-Za-z]+)?|\d+\.\d+|\d+", line[i:])
                toks.append(Tok("NUM", m.group(0), lineno))
                i += len(m.group(0))
                continue
            if c == '"' or c == "'":
                quote = c
                j = i + 1
                out = []
                while j < len(line) and line[j] != quote:
                    if line[j] == "\\" and j + 1 < len(line):
                        nxt = line[j + 1]
                        if quote == "'":
                            if nxt in ("'", "\\"):
                                out.append(nxt)
                            else:
                                out.append("\\" + nxt)
                        else:
                            out.append({"n": "\n", "t": "\t", "\\": "\\", '"': '"', "'": "'"}.get(nxt, "\\" + nxt))
                        j += 2
                        continue
                    out.append(line[j])
                    j += 1
                if j >= len(line):
                    raise SyntaxError(f"unterminated string at line {lineno}")
                toks.append(Tok("STR", "".join(out), lineno))
                i = j + 1
                continue
            if c.isalpha() or c == "_" or ord(c) > 127:
                m = re.match(r"[^\W\d]\w*", line[i:])
                word = m.group(0)
                toks.append(Tok("KW" if word in KEYWORDS else "ID", word, lineno))
                i += len(word)
                continue
            for op in OPERATORS:
                if line.startswith(op, i):
                    toks.append(Tok("OP", op, lineno))
                    i += len(op)
                    if op in "([":
                        depth += 1
                    elif op in ")]":
                        depth -= 1
                    break
            else:
                raise SyntaxError(f"unexpected character {c!r} at line {lineno}")
        if depth == 0:
            toks.append(Tok("NEWLINE", None, lineno))
    while len(indents) > 1:
        indents.pop()
        toks.append(Tok("DEDENT", None, 0))
    toks.append(Tok("EOF", None, 0))
    return toks


# ---------------------------------------------------------------- AST

class Node:
    def __init__(self, kind, **fields):
        self.kind = kind
        self.__dict__.update(fields)

    def __repr__(self):
        return f"{self.kind}({', '.join(f'{k}={v!r}' for k, v in self.__dict__.items() if k != 'kind')})"


BINARY_TIERS = [["|>"], ["or"], ["and"], ["==", "!=", "<", ">", "<=", ">="], [".."], ["+", "-"],
                ["*", "/", "%", "//", "."], ["**"]]
BINARY_PREC = {op: i for i, tier in enumerate(BINARY_TIERS) for op in tier}
RIGHT_ASSOC = {"**"}
UNARY_PREC = len(BINARY_TIERS)


class Parser:
    def __init__(self, toks):
        self.toks = toks
        self.i = 0

    def peek(self, n=0):
        return self.toks[self.i + n]

    def advance(self):
        t = self.toks[self.i]
        self.i += 1
        return t

    def at(self, kind, value=None):
        t = self.peek()
        return t.kind == kind and (value is None or t.value == value)

    def expect(self, kind, value=None):
        t = self.advance()
        if t.kind != kind or (value is not None and t.value != value):
            raise SyntaxError(f"expected {kind} {value or ''} at line {t.line}, got {t}")
        return t

    def skip_newlines(self):
        while self.at("NEWLINE"):
            self.advance()

    def program(self):
        stmts = []
        self.skip_newlines()
        while not self.at("EOF"):
            stmts.append(self.statement())
            self.skip_newlines()
        return Node("Program", body=stmts)

    def block(self):
        self.expect("NEWLINE")
        self.expect("INDENT")
        stmts = []
        self.skip_newlines()
        while not self.at("DEDENT"):
            stmts.append(self.statement())
            self.skip_newlines()
        self.expect("DEDENT")
        return stmts

    def statement(self):
        t = self.peek()
        if t.kind == "KW":
            if t.value == "let":
                self.advance()
                mutable = False
                if self.at("KW", "mut"):
                    self.advance()
                    mutable = True
                name = self.expect("ID").value
                ann = None
                if self.at("OP", ":"):
                    self.advance()
                    ann = self.advance().value
                expr = None
                if self.at("OP", "="):
                    self.advance()
                    expr = self.expression()
                self.end()
                return Node("Let", name=name, mutable=mutable, ann=ann, expr=expr, line=t.line)
            if t.value == "if":
                return self.if_stmt()
            if t.value in ("while", "until"):
                self.advance()
                cond = self.expression()
                body = self.block()
                return Node("While" if t.value == "while" else "Until", cond=cond, body=body, line=t.line)
            if t.value == "for":
                self.advance()
                var = self.expect("ID").value
                self.expect("KW", "in")
                rng = self.expression()
                if rng.kind != "Bin" or rng.op != "..":
                    raise SyntaxError(f"for loop needs a range at line {t.line}")
                body = self.block()
                return Node("For", var=var, start=rng.left, end=rng.right, body=body, line=t.line)
            if t.value == "fn":
                self.advance()
                name = self.expect("ID").value
                self.expect("OP", "(")
                params = []
                while not self.at("OP", ")"):
                    pname = self.expect("ID").value
                    pann = None
                    if self.at("OP", ":"):
                        self.advance()
                        pann = self.advance().value
                    params.append((pname, pann))
                    if self.at("OP", ","):
                        self.advance()
                self.expect("OP", ")")
                body = self.block()
                return Node("Fn", name=name, params=params, body=body, line=t.line)
            if t.value == "return":
                self.advance()
                expr = None if self.at("NEWLINE") else self.expression()
                self.end()
                return Node("Return", expr=expr, line=t.line)
            if t.value in ("break", "continue"):
                self.advance()
                self.end()
                return Node(t.value.capitalize(), line=t.line)
        # assignment or expression
        expr = self.expression()
        if self.at("OP", "="):
            self.advance()
            value = self.expression()
            self.end()
            if expr.kind == "Var":
                if expr.name == "MEMOIZATION":
                    return Node("Memo", value=value, line=t.line)
                return Node("Assign", name=expr.name, expr=value, line=t.line)
            if expr.kind == "Index":
                return Node("IndexAssign", target=expr.target, index=expr.index, expr=value, line=t.line)
            raise SyntaxError(f"bad assignment target at line {t.line}")
        self.end()
        return Node("Expr", expr=expr, line=t.line)

    def end(self):
        if self.at("NEWLINE"):
            self.advance()
        elif not self.at("EOF") and not self.at("DEDENT"):
            raise SyntaxError(f"expected end of statement at line {self.peek().line}, got {self.peek()}")

    def if_stmt(self):
        t = self.expect("KW", "if")
        cond = self.expression()
        body = self.block()
        orelse = None
        self.skip_newlines()
        if self.at("KW", "else"):
            self.advance()
            if self.at("KW", "if"):
                orelse = [self.if_stmt()]
            else:
                orelse = self.block()
        return Node("If", cond=cond, body=body, orelse=orelse, line=t.line)

    def expression(self, min_prec=0):
        left = self.unary()
        while True:
            t = self.peek()
            op = t.value if t.kind == "OP" or (t.kind == "KW" and t.value in ("and", "or")) else None
            if op not in BINARY_PREC or BINARY_PREC[op] < min_prec:
                break
            self.advance()
            prec = BINARY_PREC[op]
            if op == "|>":
                call = self.expression(prec + 1)
                if call.kind != "Call":
                    raise SyntaxError(f"pipe into a non-call at line {t.line}")
                left = Node("Call", name=call.name, args=[left] + call.args)
                continue
            right = self.expression(prec if op in RIGHT_ASSOC else prec + 1)
            left = Node("Bin", op=op, left=left, right=right)
        return left

    def unary(self):
        t = self.peek()
        if t.kind == "OP" and t.value in ("-", "!"):
            self.advance()
            return Node("Unary", op="-" if t.value == "-" else "not", operand=self.expression(UNARY_PREC))
        if t.kind == "KW" and t.value == "not":
            self.advance()
            return Node("Unary", op="not", operand=self.expression(UNARY_PREC))
        return self.postfix(self.primary())

    def postfix(self, expr):
        while True:
            if self.at("OP", "("):
                if expr.kind != "Var":
                    raise SyntaxError("call of a non-name")
                self.advance()
                args = []
                while not self.at("OP", ")"):
                    args.append(self.expression())
                    if self.at("OP", ","):
                        self.advance()
                self.expect("OP", ")")
                expr = Node("Call", name=expr.name, args=args)
            elif self.at("OP", "["):
                self.advance()
                index = self.expression()
                self.expect("OP", "]")
                expr = Node("Index", target=expr, index=index)
            else:
                return expr

    def primary(self):
        t = self.advance()
        if t.kind == "NUM":
            return number_node(t.value)
        if t.kind == "STR":
            return Node("Str", value=t.value)
        if t.kind == "KW":
            if t.value in ("true", "false"):
                return Node("Bool", value=t.value == "true")
            if t.value == "null":
                return Node("Null")
            if t.value == "extern":
                self.expect("OP", "(")
                args = []
                while not self.at("OP", ")"):
                    args.append(self.expression())
                    if self.at("OP", ","):
                        self.advance()
                self.expect("OP", ")")
                return Node("Extern", args=args)
            raise SyntaxError(f"unexpected keyword {t.value} at line {t.line}")
        if t.kind == "ID":
            return Node("Var", name=t.value)
        if t.kind == "OP" and t.value == "(":
            inner = self.expression()
            self.expect("OP", ")")
            return Node("Group", expr=inner)
        if t.kind == "OP" and t.value == "[":
            items = []
            while not self.at("OP", "]"):
                items.append(self.expression())
                if self.at("OP", ","):
                    self.advance()
            self.expect("OP", "]")
            return Node("Array", items=items)
        raise SyntaxError(f"unexpected token {t} at line {t.line}")


def number_node(text):
    """A numeric literal as an exact value, remembering how it was written."""
    if "@" in text:
        base_text, digits = text.split("@")
        base = int(base_text)
        whole, _, frac = digits.partition(".")
        value = Fraction(int(whole, base))
        for k, d in enumerate(frac, 1):
            value += Fraction(int(d, base), base ** k)
        return Node("Num", value=value, real="." in digits, base=base, text=text)
    if "." in text:
        return Node("Num", value=Fraction(text), real=True, base=10, text=text)
    return Node("Num", value=Fraction(int(text)), real=False, base=10, text=text)


def parse(source):
    return Parser(lex(source)).program()


# ---------------------------------------------------------------- library

LIB_FILES = ["render.lm", "value_to_string.lm", "string_to_value.lm", "numeric.lm", "array.lm", "string.lm", "string_ord_chr.lm",
             "factorial.lm", "round.lm", "e_integer.lm", "pi_machin.lm", "modular_arithmetic.lm",
             "primes.lm", "number_theory.lm", "constants_1024.lm", "constants.lm", "constants_default.lm"]

KERNEL_BUILTINS = {
    "len": "builtin.len", "char_at": "builtin.char_at", "ord": "builtin.ord", "chr": "builtin.chr",
    "kind": "builtin.typeof", "error": "builtin.error", "real": "builtin.real", "precision": "builtin.precision",
    "num": "builtin.num", "den": "builtin.den", "push": "builtin.push", "emit": "builtin.emit",
}
# Library renderers a language spells with one polymorphic conversion instead.
POLYMORPHIC = {"int_to_string": "builtin.to_string", "real_to_string": "builtin.to_string",
               "rational_to_string": "builtin.to_string", "bool_to_string": "builtin.to_string",
               "null_to_string": "builtin.to_string", "array_to_string": "builtin.to_string",
               "value_to_string": "builtin.to_string", "real_default": "builtin.to_real", "int": "builtin.to_int"}
SYSTEM_NAMES = {"ARGS", "MEMOIZATION", "REAL_DEFAULT_PRECISION", "INTEGER", "RATIONAL", "REAL",
                "STRING", "BOOLEAN", "ARRAY", "NULL"}


def load_library():
    """Every library function by name, with the top-level assignments of its
    file, and every library constant by name."""
    fns = {}
    constants = {}
    for name in LIB_FILES:
        prog = parse((LIB / name).read_text(encoding="utf-8"))
        globals_ = [s for s in prog.body if s.kind in ("Assign", "Let")]
        for g in globals_:
            constants[g.name] = g
        for s in prog.body:
            if s.kind == "Fn":
                fns[s.name] = (s, globals_)
    return fns, constants


def walk(node):
    """Every node under `node`, itself included."""
    yield node
    for value in list(node.__dict__.values()):
        if isinstance(value, Node):
            yield from walk(value)
        elif isinstance(value, list):
            for item in value:
                if isinstance(item, Node):
                    yield from walk(item)


def called_names(nodes):
    names = set()
    for n in nodes:
        for m in walk(n):
            if m.kind == "Call":
                names.add(m.name)
    return names


def free_names(nodes):
    names = set()
    for n in nodes:
        for m in walk(n):
            if m.kind == "Var":
                names.add(m.name)
    return names


def library_closure(program, lib, constants, replaced):
    """The library functions the program needs, in definition order, with
    the library constants the program or those functions read. A function
    in `replaced` is spelled by a builtin of the target and is not ported."""
    defined = {s.name for s in program.body if s.kind == "Fn"}
    needed = []
    seen = set()
    wanted = lambda n: n in lib and n not in defined and n not in replaced
    pending = [n for n in called_names(program.body) if wanted(n)]
    while pending:
        name = pending.pop()
        if name in seen:
            continue
        seen.add(name)
        fn, _ = lib[name]
        needed.append(fn)
        for callee in called_names([fn]):
            if wanted(callee) and callee not in seen:
                pending.append(callee)
    order = {name: i for i, name in enumerate(lib)}
    needed.sort(key=lambda f: order[f.name])
    assigned = {s.name for s in walk(Node("B", body=program.body)) if s.kind in ("Assign", "Let")}
    free = free_names(program.body + needed)
    globals_ = [g for name, g in constants.items() if name in free and name not in assigned]
    return globals_, needed


# ---------------------------------------------------------------- kinds

INT, REAL, RATIONAL, BOOL, STR, ARRAY, NULL, UNKNOWN = "int", "real", "rational", "bool", "str", "array", "null", "?"


class Kinds:
    """Best-effort value kinds for variables and function results, used only
    to choose type words and print formats where a language needs them."""

    def __init__(self, program_body):
        self.fn_returns = {}
        self.fn_params = {}
        self.fns = {s.name: s for s in program_body if s.kind == "Fn"}
        self.program_body = program_body
        for _ in range(3):
            self.pass_over()

    def pass_over(self):
        env = {}
        self.infer_body(self.program_body, env)

    def infer_body(self, body, env):
        for s in body:
            if s.kind in ("Assign", "Let") and s.expr is not None:
                env[s.name] = self.kind_of(s.expr, env)
            elif s.kind == "Let":
                env[s.name] = NULL
            elif s.kind == "For":
                env[s.var] = INT
                self.infer_body(s.body, env)
            elif s.kind in ("While", "Until"):
                self.infer_body(s.body, env)
            elif s.kind == "If":
                self.infer_body(s.body, env)
                if s.orelse:
                    self.infer_body(s.orelse, env)
            elif s.kind == "Fn":
                params = self.fn_params.setdefault(s.name, [UNKNOWN] * len(s.params))
                fenv = dict(env)
                for (p, _), k in zip(s.params, params):
                    fenv[p] = k
                self.infer_body(s.body, fenv)
                returns = [self.kind_of(r.expr, fenv) if r.expr else NULL for r in walk_returns(s.body)]
                tail = tail_exprs(s.body)
                returns += [self.kind_of(e, fenv) for e in tail]
                kinds = {k for k in returns if k != UNKNOWN}
                self.fn_returns[s.name] = kinds.pop() if len(kinds) == 1 else (NULL if not returns else UNKNOWN)
            elif s.kind == "Expr":
                self.kind_of(s.expr, env)
            elif s.kind == "Return" and s.expr is not None:
                self.kind_of(s.expr, env)

    def kind_of(self, e, env):
        if e.kind == "Num":
            return REAL if e.real else INT
        if e.kind == "Str":
            return STR
        if e.kind == "Bool":
            return BOOL
        if e.kind == "Null":
            return NULL
        if e.kind == "Array":
            return ARRAY
        if e.kind == "Group":
            return self.kind_of(e.expr, env)
        if e.kind == "Var":
            return env.get(e.name, UNKNOWN)
        if e.kind == "Index":
            return UNKNOWN
        if e.kind == "Unary":
            return BOOL if e.op == "not" else self.kind_of(e.operand, env)
        if e.kind == "Bin":
            l, r = self.kind_of(e.left, env), self.kind_of(e.right, env)
            if e.op in ("==", "!=", "<", ">", "<=", ">=", "and", "or"):
                return BOOL
            if e.op == ".":
                return STR
            if e.op == "+" and STR in (l, r):
                return STR
            if e.op == "/":
                return REAL if REAL in (l, r) else RATIONAL
            if e.op in ("//", "%"):
                return REAL if REAL in (l, r) else INT
            if REAL in (l, r):
                return REAL
            if UNKNOWN in (l, r):
                return UNKNOWN
            if RATIONAL in (l, r):
                return RATIONAL
            return INT
        if e.kind == "Call":
            args = [self.kind_of(a, env) for a in e.args]
            if e.name in self.fns:
                params = self.fn_params.setdefault(e.name, [UNKNOWN] * len(self.fns[e.name].params))
                for i, k in enumerate(args[:len(params)]):
                    if params[i] == UNKNOWN:
                        params[i] = k
                return self.fn_returns.get(e.name, UNKNOWN)
            return {"len": INT, "ord": INT, "chr": STR, "char_at": STR, "int_to_string": STR,
                    "real_to_string": STR, "kind_to_string": STR, "value_to_string": STR, "int": INT,
                    "frac": REAL, "real": REAL, "real_default": REAL, "num": INT, "den": INT,
                    "range": ARRAY}.get(e.name, UNKNOWN)
        return UNKNOWN


def walk_returns(body):
    for s in body:
        if s.kind == "Return":
            yield s
        elif s.kind in ("If",):
            yield from walk_returns(s.body)
            if s.orelse:
                yield from walk_returns(s.orelse)
        elif s.kind in ("While", "Until", "For"):
            yield from walk_returns(s.body)


def tail_exprs(body):
    """The expressions a function body yields as its value when it ends."""
    if not body:
        return []
    last = body[-1]
    if last.kind == "Expr" and not (last.expr.kind == "Call" and last.expr.name in ("print", "write")):
        return [last.expr]
    if last.kind == "If":
        found = tail_exprs(last.body)
        if last.orelse:
            found += tail_exprs(last.orelse)
        return found
    return []


# ---------------------------------------------------------------- emitting

TYPE_WORDS = {
    # One word per type: the kernels read a type name as a single word.
    "rust": {INT: "i64", REAL: "f64", BOOL: "bool", STR: "String", RATIONAL: "f64", UNKNOWN: "i64"},
    "swift": {INT: "Int", REAL: "Double", BOOL: "Bool", STR: "String", RATIONAL: "Double", UNKNOWN: "Int"},
    "pascal": {INT: "integer", REAL: "real", BOOL: "boolean", STR: "string", RATIONAL: "real", UNKNOWN: "integer"},
    "c": {INT: "long", REAL: "double", BOOL: "bool", UNKNOWN: "long"},
}
C_FORMATS = {INT: "%ld", REAL: "%f", STR: "%s", BOOL: "%d", UNKNOWN: "%ld", RATIONAL: "%f"}

# Builtin names a language writes with the receiver first, `arr.push(x)`:
# the pipe spelled `.`. A name not listed is called as a function (Python's
# len(s)). This is how each language is written, not something a definition
# says; the kernels accept both forms.
METHOD_FORM = {
    "python": {"append"},
    "javascript": {"push", "length", "charAt"},
    "ruby": {"push", "length", "size", "to_s", "to_i", "to_f", "ord", "chr"},
    "swift": {"append", "count"},
    "rust": {"push", "len", "to_string"},
}
# Where one kernel builtin has a name per kind of argument (PHP's strlen
# and count), the name to use for each kind.
LEN_BY_KIND = {"php": {STR: "strlen", ARRAY: "count"}}
# Method names written without a call bracket: properties.
PROPERTY_FORM = {
    "javascript": {"length"},
    "swift": {"count"},
    "ruby": {"length", "size", "to_s", "to_i", "to_f", "ord", "chr"},
}


class Emitter:
    def __init__(self, definition):
        self.d = definition
        self.name = definition["language"]
        self.style = definition["block.style"]
        self.indent_unit = " " * (definition["block.indent_size"] or 4)
        self.tiers = definition["op.precedence"]
        self.reserved = self.collect_reserved()
        self.type_first = definition["stmt.let.type_first"]
        self.prefix = (definition["identifier.variable_prefix"] or [None])[0]
        self.case_insensitive = definition["identifier.case_insensitive"]

    # ---- definition queries
    def has(self, label):
        return bool(self.d[label])

    def w(self, label, what=None):
        if not self.d[label]:
            raise Skip(f"no `{what or label}`")
        return self.d[label][0]

    def collect_reserved(self):
        words = set()
        for label, value in self.d.items():
            if label.startswith("$comment") or not isinstance(value, list) or label == "op.precedence":
                continue
            if label == "system.entry":
                continue  # the entry function keeps its name
            if label.startswith(("stmt.", "literal.", "op.", "block.", "stack.", "builtin.", "system.")):
                for s in value:
                    if isinstance(s, str) and re.fullmatch(r"[^\W\d]\w*", s):
                        words.add(s)
        return words

    def binary_tier(self, lexeme):
        for i, tier in enumerate(self.tiers):
            if lexeme in tier:
                return i
        raise Skip(f"`{lexeme}` has no tier")

    def unary_tier(self, lexeme):
        for i in range(len(self.tiers) - 1, -1, -1):
            if lexeme in self.tiers[i]:
                return i
        raise Skip(f"`{lexeme}` has no tier")

    # ---- names
    def ident(self, name, is_function=False):
        """A Lumen identifier in the target: renamed on a collision with a
        reserved word or builtin name, prefixed when variables carry one."""
        out = name
        if out in self.reserved or (self.case_insensitive and out.lower() in {r.lower() for r in self.reserved}):
            out = out + "_"
        if not is_function and self.prefix:
            out = self.prefix + out
        return out

    # ---- literals
    def string(self, value):
        quotes = self.d["lexical.string_quotes"]
        raw = self.d["lexical.raw_quotes"]
        escapes = self.d["lexical.string_escapes"]
        choices = [q for q in quotes if q not in raw] + [q for q in quotes if q in raw]
        if not choices:
            raise Skip("no string quotes")
        for q in choices:
            is_raw = q in raw
            out = []
            ok = True
            for c in value:
                if c == "\\":
                    out.append("\\\\")
                elif c == q:
                    if not is_raw and (q in escapes or "\\" in escapes):
                        out.append("\\" + q)
                    else:
                        ok = False
                        break
                elif c == "\n":
                    if not is_raw and "n" in escapes:
                        out.append("\\n")
                    else:
                        ok = False
                        break
                elif c == "\t":
                    if not is_raw and "t" in escapes:
                        out.append("\\t")
                    else:
                        ok = False
                        break
                else:
                    out.append(c)
            if ok:
                return q + "".join(out) + q
        raise Skip("a string the language cannot spell")

    def number(self, node):
        if node.real:
            if node.base == 10 and "@" not in node.text:
                return node.text
            v = node.value
            den = v.denominator
            while den % 2 == 0:
                den //= 2
            while den % 5 == 0:
                den //= 5
            if den != 1:
                raise Skip("a base-N real with no finite decimal form")
            return decimal_of(v)
        if node.base == 16 and self.has("lexical.number.hex_prefix"):
            return self.d["lexical.number.hex_prefix"][0] + format(int(node.value), "X")
        return str(int(node.value))

    # ---- expressions
    OP_LABELS = {"+": "op.add", "-": "op.sub", "*": "op.mul", "/": "op.div", "//": "op.quot", "%": "op.rem",
                 "**": "op.pow", "==": "op.eq", "!=": "op.ne", "<": "op.lt", "<=": "op.le", ">": "op.gt",
                 ">=": "op.ge", "and": "op.and", "or": "op.or", ".": "op.concat", "..": "op.range"}

    def expr(self, e, scope):
        """The expression's text and its target precedence tier (None for atoms)."""
        if e.kind == "Num":
            return self.number(e), None
        if e.kind == "Str":
            return self.string(e.value), None
        if e.kind == "Bool":
            return self.w("literal.true" if e.value else "literal.false"), None
        if e.kind == "Null":
            return self.w("literal.null"), None
        if e.kind == "Var":
            if e.name in SYSTEM_NAMES:
                raise Skip(f"no `{e.name}`")
            return self.ident(e.name), None
        if e.kind == "Group":
            text, _ = self.expr(e.expr, scope)
            return self.w("syntax.group.open") + text + self.w("syntax.group.close"), None
        if e.kind == "Array":
            open_, close = self.w("syntax.array.open", "array literal"), self.w("syntax.array.close")
            sep = self.w("syntax.array.separator") + " "
            return open_ + sep.join(self.expr(i, scope)[0] for i in e.items) + close, None
        if e.kind == "Index":
            target, _ = self.expr(e.target, scope)
            index, _ = self.expr(e.index, scope)
            return target + self.w("op.index.open", "indexing") + index + self.w("op.index.close"), None
        if e.kind == "Extern":
            raise Skip("no `extern`")
        if e.kind == "Unary":
            label = "op.negate" if e.op == "-" else "op.not"
            lexeme = self.w(label)
            tier = self.unary_tier(lexeme)
            text, child_tier = self.expr(e.operand, scope)
            if child_tier is not None and child_tier < tier:
                text = self.paren(text)
            joiner = " " if re.fullmatch(r"\w+", lexeme) else ""
            return lexeme + joiner + text, tier
        if e.kind == "Bin":
            op = e.op
            if op == "." and not self.has("op.concat"):
                op = "+"
            lexeme = self.w(self.OP_LABELS[op], e.op)
            tier = self.binary_tier(lexeme)
            right_assoc = lexeme in self.d["op.right_associative"]
            left, lt = self.expr(e.left, scope)
            right, rt = self.expr(e.right, scope)
            if lt is not None and (lt < tier or (lt == tier and right_assoc)):
                left = self.paren(left)
            if rt is not None and (rt < tier or (rt == tier and not right_assoc)):
                right = self.paren(right)
            return f"{left} {lexeme} {right}", tier
        if e.kind == "Call":
            return self.call(e, scope), None
        raise Skip(f"unexpected {e.kind}")

    def paren(self, text):
        return self.w("syntax.group.open") + text + self.w("syntax.group.close")

    def arguments(self, args, scope, param_names=None):
        open_, close = self.w("syntax.call.open"), self.w("syntax.call.close")
        sep = self.w("syntax.call.separator") + " " if len(args) > 1 else ""
        parts = []
        for i, a in enumerate(args):
            text, _ = self.expr(a, scope)
            if param_names is not None and self.has("syntax.call.label") and i < len(param_names):
                text = f"{param_names[i]}{self.d['syntax.call.label'][0]} {text}"
            parts.append(text)
        return open_ + sep.join(parts) + close

    def call(self, e, scope):
        name = e.name
        if name in ("print", "write"):
            raise Skip("`print` as a value")
        if name in POLYMORPHIC and self.has(POLYMORPHIC[name]):
            # str(x) where Lumen's library spells each kind's renderer
            return self.builtin_call(self.d[POLYMORPHIC[name]][0], e.args, scope)
        if name in scope.functions:
            labels = [p for p, _ in scope.functions[name].params]
            return self.ident(name, True) + self.arguments(e.args, scope, labels)
        if name == "char_at" and not self.has("builtin.char_at") and self.d["op.index.strings"] and len(e.args) == 2:
            # s[i] where the language indexes into strings
            target, tier = self.expr(e.args[0], scope)
            if tier is not None:
                target = self.paren(target)
            index, _ = self.expr(e.args[1], scope)
            return target + self.w("op.index.open") + index + self.w("op.index.close")
        if name in KERNEL_BUILTINS:
            builtin = self.w(KERNEL_BUILTINS[name], name)
            if name == "len" and self.name in LEN_BY_KIND and e.args:
                kind = self.kinds.kind_of(e.args[0], scope.kinds_env)
                chosen = LEN_BY_KIND[self.name].get(kind)
                if chosen in self.d["builtin.len"]:
                    builtin = chosen
            return self.builtin_call(builtin, e.args, scope)
        raise Skip(f"unknown function `{name}`")

    def builtin_call(self, builtin, args, scope):
        """A builtin call as the language writes it: a function call, or a
        method call on the first argument where that is the language's form."""
        if builtin in METHOD_FORM.get(self.name, set()) and args:
            receiver, tier = self.expr(args[0], scope)
            if tier is not None:
                receiver = self.paren(receiver)
            dot = self.w("op.pipe")
            if builtin in PROPERTY_FORM.get(self.name, set()) and len(args) == 1:
                return f"{receiver}{dot}{builtin}"
            return f"{receiver}{dot}{builtin}{self.arguments(args[1:], scope)}"
        return builtin + self.arguments(args, scope)

    # ---- statements
    def program(self, prog, lib_globals, lib_fns, source_path):
        scope = Scope(prog.body, lib_fns)
        self.kinds = Kinds(lib_globals + lib_fns + prog.body)
        self.check_globals(prog, lib_globals, lib_fns)
        lines = []
        comment = self.d["lexical.comment_line"]
        if comment:
            lines.append(f"{comment[0]} {HEADER.format(source=source_path)}")
        prologue = self.d["lexical.prologue"][0] if self.d["lexical.prologue"] else None
        body = lib_globals + lib_fns + prog.body
        entry = self.d["system.entry"]
        if entry:
            # Functions at the top level, everything else inside the entry function.
            fns = [s for s in body if s.kind == "Fn"]
            rest = [s for s in body if s.kind != "Fn"]
            for f in fns:
                lines.extend(self.statement(f, scope, 0))
                lines.append("")
            main = Node("Fn", name=entry[0], params=[], body=rest + ([Node("Return", expr=Node("Num", value=Fraction(0), real=False, base=10, text="0"))] if self.type_first else []), line=0)
            scope.functions[entry[0]] = main
            self.kinds.fn_returns[entry[0]] = INT if self.type_first else NULL
            lines.extend(self.statement(main, scope, 0, is_entry=True))
        elif self.name == "pascal":
            lines.extend(self.pascal_program(body, scope))
        else:
            lines.extend(self.hoist(body, scope, 0))
            for s in body:
                lines.extend(self.statement(s, scope, 0))
                if s.kind == "Fn":
                    lines.append("")
        text = "\n".join(lines).rstrip("\n") + "\n"
        if prologue is not None:
            # An import prologue (Python's `import sys`) only when the module is used.
            module = prologue.split()[1] if prologue.startswith("import ") else None
            if module is None or module + "." in text:
                text = prologue + "\n" + text
        return text

    def check_globals(self, prog, lib_globals, lib_fns):
        """A function reading a program-level variable needs the language to
        share top-level names with functions; Python, JavaScript, Swift and
        Pascal do. Elsewhere a library constant is inlined into the function
        that reads it, and an example's own variable is a reason to skip."""
        self.inline = {}
        if self.name in ("python", "javascript", "swift", "pascal"):
            return
        constants = {g.name: g for g in lib_globals}
        # A program-level name assigned once, to a literal, is a constant too.
        counts = assignment_counts(prog.body)
        for st in prog.body:
            if st.kind in ("Assign", "Let") and st.expr is not None and st.expr.kind in ("Str", "Num", "Bool") and counts.get(st.name) == 1:
                constants[st.name] = st
        top = {s.name for s in prog.body + lib_globals if s.kind in ("Assign", "Let")}
        for f in [s for s in prog.body if s.kind == "Fn"] + lib_fns:
            params = {p for p, _ in f.params}
            assigned = {s.name for s in walk(Node("B", body=f.body)) if s.kind in ("Assign", "Let")}
            for n in sorted(free_names(f.body) & top):
                if n in params or n in assigned:
                    continue
                if n in constants:
                    self.inline.setdefault(f.name, []).append(constants[n])
                else:
                    raise Skip(f"function `{f.name}` reads program-level `{n}`")

    def statements(self, body, scope, depth):
        lines = []
        for s in body:
            lines.extend(self.statement(s, scope, depth))
        return lines

    def terminated(self, text):
        t = self.d["stmt.terminator"]
        return text + t[0] if t and text and not text.endswith(t[0]) else text

    def indent(self, depth):
        return self.indent_unit * depth

    def block(self, header, body, scope, depth, then_word=None, closer_tail=""):
        """`header` and a block of `body` in the target's block style."""
        ind = self.indent(depth)
        inner = self.statements(body, scope, depth + 1)
        if not inner and self.has("stmt.pass"):
            inner = [self.indent(depth + 1) + self.d["stmt.pass"][0]]
        if self.style == "indentation":
            intro = self.d["block.intro"]
            return [ind + header + (intro[0] if intro else "")] + inner, None
        if self.style == "braces":
            opener, closer = self.d["block.open"][0], self.d["block.close"][0]
            intro = (" " + then_word) if then_word else ""
            return [ind + header + intro + " " + opener] + inner, ind + closer
        # keyword style: header [intro] ... end
        intro = (" " + then_word) if then_word and then_word in self.d["block.intro"] else ""
        return [ind + header + intro] + inner, ind + self.d["block.close"][0]

    def close(self, lines, closer, depth, follow=""):
        if closer is not None:
            lines.append(closer + follow)
        return lines

    def statement(self, s, scope, depth, is_entry=False):
        ind = self.indent(depth)
        k = s.kind
        if k == "Expr":
            e = s.expr
            if e.kind == "Call" and e.name in ("print", "write"):
                return [ind + self.terminated(self.output(e, scope))]
            text, _ = self.expr(e, scope)
            return [ind + self.terminated(text)]
        if k in ("Assign", "Let"):
            return [ind + self.terminated(self.binding(s, scope, depth))]
        if k == "IndexAssign":
            target, _ = self.expr(s.target, scope)
            index, _ = self.expr(s.index, scope)
            value, _ = self.expr(s.expr, scope)
            return [ind + self.terminated(f"{target}{self.w('op.index.open', 'indexing')}{index}{self.w('op.index.close')} {self.w('stmt.assign')} {value}")]
        if k == "Memo":
            raise Skip("no memoization switch")
        if k == "Return":
            word = self.w("stmt.return")
            if s.expr is None:
                return [ind + self.terminated(word)]
            text, _ = self.expr(s.expr, scope)
            if self.name == "pascal":
                return [ind + self.terminated(f"{word}({text})")]
            return [ind + self.terminated(f"{word} {text}")]
        if k == "Break":
            return [ind + self.terminated(self.w("stmt.break"))]
        if k == "Continue":
            return [ind + self.terminated(self.w("stmt.continue"))]
        if k == "If":
            return self.if_stmt(s, scope, depth)
        if k in ("While", "Until"):
            cond = s.cond
            if k == "Until":
                if self.has("stmt.until"):
                    word = self.d["stmt.until"][0]
                else:
                    word = self.w("stmt.while")
                    cond = Node("Unary", op="not", operand=cond)
            else:
                word = self.w("stmt.while")
            text, _ = self.expr(cond, scope)
            with scope.declaring(s.body):
                lines, closer = self.block(f"{word} {self.condition(text)}", s.body, scope, depth, then_word=self.intro("do"))
            return self.close(lines, closer, depth, self.block_end())
        if k == "For":
            return self.for_loop(s, scope, depth)
        if k == "Fn":
            return self.function(s, scope, depth, is_entry)
        raise Skip(f"unexpected statement {k}")

    def condition(self, text):
        """C-family languages parenthesise conditions; the rest do not."""
        if self.name in ("c", "javascript", "php"):
            return self.paren(text)
        return text

    def intro(self, word):
        return word if word in self.d["block.intro"] else None

    def block_end(self):
        """What follows a block's closer: Pascal's `end;`."""
        t = self.d["stmt.terminator"]
        return t[0] if self.style == "braces" and self.d["block.close"][0].isalpha() and t else ""

    def if_stmt(self, s, scope, depth, word=None):
        word = word or self.w("stmt.if")
        text, _ = self.expr(s.cond, scope)
        with scope.declaring(s.body):
            lines, closer = self.block(f"{word} {self.condition(text)}", s.body, scope, depth, then_word=self.intro("then"))
        orelse = s.orelse
        if orelse is None:
            return self.close(lines, closer, depth, self.block_end())
        else_word = self.w("stmt.else")
        chained = len(orelse) == 1 and orelse[0].kind == "If"
        if chained:
            if self.has("stmt.elif"):
                head = self.d["stmt.elif"][0]
                if closer is not None and self.style != "keyword":
                    lines.append(closer)
                    rest = self.if_stmt(orelse[0], scope, depth, word=head)
                    rest[0] = rest[0].lstrip()
                    lines[-1] += " " + rest[0]
                    return lines + rest[1:]
                rest = self.if_stmt(orelse[0], scope, depth, word=head)
                return lines + rest
            # else if
            if self.style == "keyword":
                rest = self.if_stmt(orelse[0], scope, depth, word=self.w("stmt.if"))
                return lines + [self.indent(depth) + else_word] + rest
            if closer is not None:
                lines.append(closer)
                rest = self.if_stmt(orelse[0], scope, depth, word=f"{else_word} {self.w('stmt.if')}")
                lines[-1] += " " + rest[0].lstrip()
                return lines + rest[1:]
            rest = self.if_stmt(orelse[0], scope, depth, word=f"{else_word} {self.w('stmt.if')}")
            return lines + rest
        with scope.declaring(orelse):
            else_lines, else_closer = self.block(else_word, orelse, scope, depth)
        if self.style == "indentation":
            return lines + else_lines
        if self.style == "keyword":
            return lines + else_lines + [else_closer]
        lines.append(closer + " " + else_lines[0].lstrip())
        return self.close(lines + else_lines[1:], else_closer, depth, self.block_end())

    def for_loop(self, s, scope, depth):
        start, _ = self.expr(s.start, scope)
        end, _ = self.expr(s.end, scope)
        var = self.ident(s.var)
        if self.has("stmt.for") and self.has("stmt.for.in"):
            if self.has("op.range"):
                rng = f"{start}{self.d['op.range'][0]}{end}"
            elif self.has("builtin.range"):
                rng = f"{self.d['builtin.range'][0]}({start}, {end})"
            else:
                raise Skip("no range")
            scope.declare(s.var)
            with scope.declaring(s.body):
                lines, closer = self.block(f"{self.d['stmt.for'][0]} {var} {self.d['stmt.for.in'][0]} {rng}", s.body, scope, depth, then_word=self.intro("do"))
            return self.close(lines, closer, depth, self.block_end())
        # A while loop with an explicit counter; the step also runs before
        # each `continue` of this loop, which would otherwise skip it.
        step = Node("Assign", name=s.var, expr=Node("Bin", op="+", left=Node("Var", name=s.var), right=Node("Num", value=Fraction(1), real=False, base=10, text="1")), line=s.line)
        init = Node("Assign", name=s.var, expr=s.start, line=s.line)
        cond = Node("Bin", op="<", left=Node("Var", name=s.var), right=s.end)
        lines = self.statement(init, scope, depth)
        loop = Node("While", cond=cond, body=step_before_continue(s.body, step) + [step], line=s.line)
        return lines + self.statement(loop, scope, depth)

    def binding(self, s, scope, depth):
        """An assignment, or a declaration where the language declares."""
        name = self.ident(s.name)
        assign = self.w("stmt.assign")
        if s.expr is None:
            value = None
        else:
            value, _ = self.expr(s.expr, scope)
        kind = self.kinds.kind_of(s.expr, scope.kinds_env) if s.expr is not None else NULL
        if s.expr is not None:
            scope.kinds_env[s.name] = kind
        if not self.has("stmt.let") or scope.is_declared(s.name):
            if value is None:
                raise Skip("a declaration without a value")
            return f"{name} {assign} {value}"
        scope.declare(s.name)
        if self.type_first:
            type_word = self.c_type(kind)
            return f"{type_word} {name}" + (f" {assign} {value}" if value is not None else "")
        keyword = self.d["stmt.let"][0]
        mutable = scope.reassigned(s.name) or (s.kind == "Let" and s.mutable)
        if self.name == "swift":
            keyword = "var" if mutable else "let"
        elif self.name == "javascript":
            keyword = "let" if mutable else "const"
        elif self.has("stmt.let.mutable") and mutable:
            keyword += " " + self.d["stmt.let.mutable"][0]
        ann = ""
        if s.kind == "Let" and s.ann is not None and self.has("stmt.let.annotation"):
            ann = f"{self.d['stmt.let.annotation'][0]} {self.type_word(kind)}"
        if value is None:
            if self.has("stmt.let.annotation") and self.name != "rust":
                ann = f"{self.d['stmt.let.annotation'][0]} {self.type_word(kind)}"
            return f"{keyword} {name}{ann}"
        return f"{keyword} {name}{ann} {assign} {value}"

    def type_word(self, kind):
        table = TYPE_WORDS.get(self.name)
        if table is None:
            raise Skip("no type words")
        if kind not in table:
            raise Skip(f"no type word for a {kind}")
        return table[kind]

    def c_type(self, kind):
        if kind in (STR, ARRAY, NULL, RATIONAL):
            raise Skip(f"C has no spelling here for a variable holding a {kind}")
        return TYPE_WORDS["c"][kind]

    def function(self, s, scope, depth, is_entry=False):
        ind = self.indent(depth)
        name = self.ident(s.name, True)
        inner = FunctionScope(scope, s)
        params = []
        param_kinds = self.kinds.fn_params.get(s.name, [UNKNOWN] * len(s.params))
        ret_kind = self.kinds.fn_returns.get(s.name, UNKNOWN)
        if self.name == "pascal":
            return self.pascal_function(s, scope, depth, param_kinds, ret_kind)
        if self.type_first:
            ret_word = "void" if ret_kind == NULL else self.c_type(ret_kind)
            for (p, _), k in zip(s.params, param_kinds):
                params.append(f"{self.c_type(k)} {self.ident(p)}")
            header = f"{ret_word} {name}({', '.join(params) or 'void'})"
        else:
            keyword = self.w("stmt.function", "functions")
            for (p, _), k in zip(s.params, param_kinds):
                text = self.ident(p)
                if self.has("stmt.let.annotation") and self.name in TYPE_WORDS and self.name != "pascal":
                    text += f"{self.d['stmt.let.annotation'][0]} {self.type_word(k)}"
                params.append(text)
            header = f"{keyword} {name}({', '.join(params)})"
            if self.has("stmt.function.returns") and ret_kind != NULL and not is_entry:
                header += f" {self.d['stmt.function.returns'][0]} {self.type_word(ret_kind)}"
        if is_entry and self.type_first:
            header = f"int {name}(void)"
        body = explicit_returns(s.body) if not is_entry else s.body
        body = self.inline.get(s.name, []) + body
        hoisted = self.hoist(body, inner, depth + 1)
        lines, closer = self.block(header, body, inner, depth)
        lines[1:1] = hoisted
        return self.close(lines, closer, depth, self.block_end())

    def hoist(self, body, scope, depth):
        """Declarations for names first assigned inside a nested block, in
        languages whose declarations are block-scoped; the assignment that
        follows then finds the name declared."""
        if not self.has("stmt.let") or self.name == "pascal":
            return []
        lines = []
        for name in hoisted_names(body):
            if scope.is_declared(name):
                continue
            scope.declare(name)
            ident = self.ident(name)
            if self.type_first:
                lines.append(self.indent(depth) + self.terminated(f"{self.c_type(UNKNOWN)} {ident}"))
            elif self.name == "rust":
                lines.append(self.indent(depth) + self.terminated(f"let mut {ident}"))
            elif self.name == "swift":
                lines.append(self.indent(depth) + f"var {ident}: {self.type_word(UNKNOWN)}")
            else:
                lines.append(self.indent(depth) + self.terminated(f"{self.d['stmt.let'][0]} {ident}"))
        return lines

    def output(self, e, scope):
        """print(x) and write(x) in the target's builtins."""
        if len(e.args) != 1:
            raise Skip("print with several arguments")
        arg = e.args[0]
        kind = self.kinds.kind_of(arg, scope.kinds_env)
        newline = e.name == "print"
        if self.type_first:
            # C: puts for a string with a newline, printf otherwise.
            if arg.kind == "Str" and newline:
                return f"{self.w('builtin.print')}({self.string(arg.value)})"
            text, _ = self.expr(arg, scope)
            if arg.kind == "Str":
                return f"{self.w('builtin.write')}({text})"
            fmt = C_FORMATS.get(kind)
            if fmt is None:
                raise Skip(f"C has no printf format for a {kind}")
            return f"{self.w('builtin.write')}({self.string(fmt + (chr(10) if newline else ''))}, {text})"
        text, tier = self.expr(arg, scope)
        if self.has("builtin.print.placeholder") and self.has("builtin.print"):
            # Rust: println!("{}", x), or the literal alone.
            builtin = self.w("builtin.print" if newline else "builtin.write")
            if arg.kind == "Str":
                return f"{builtin}({text})"
            return f"{builtin}({self.string(self.d['builtin.print.placeholder'][0])}, {text})"
        if newline:
            if self.has("builtin.print"):
                return f"{self.d['builtin.print'][0]}({text})"
            # PHP: print writes; append the newline.
            write = self.w("builtin.write", "print")
            if arg.kind == "Str":
                return f"{write}({self.string(arg.value + chr(10))})"
            concat = self.w("op.concat", "print")
            if tier is not None and tier <= self.binary_tier(concat):
                text = self.paren(text)
            return f"{write}({text} {concat} {self.string(chr(10))})"
        if not self.has("builtin.write"):
            # write derived as Lumen's library derives it: emit of the
            # value's text, where the language has the string-only emit.
            emit = self.w("builtin.emit", "write")
            if arg.kind == "Str" or kind == STR:
                return f"{emit}({text})"
            return f"{emit}({self.w('builtin.to_string', 'write')}({text}))"
        return f"{self.d['builtin.write'][0]}({text})"

    # ---- Pascal: declarations first, then the main block
    def pascal_program(self, body, scope):
        """Program-level variables first, then the functions, then the main block."""
        main = [s for s in body if s.kind != "Fn"]
        names = self.pascal_names(main, set())
        env = {}
        self.kinds.infer_body(body, env)
        lines = []
        for n in names:
            lines.append(f"var {self.ident(n)}: {self.type_word(env.get(n, UNKNOWN))};")
            scope.declare(n)
        if names:
            lines.append("")
        for s in body:
            if s.kind == "Fn":
                lines.extend(self.statement(s, scope, 0))
                lines.append("")
        lines.append("begin")
        for s in main:
            lines.extend(self.statement(s, scope, 1))
        lines.append("end.")
        return lines

    def pascal_names(self, body, exclude):
        """Every name assigned in `body`, for a var section; Pascal ignores case."""
        names = []
        for m in walk(Node("B", body=body)):
            if m.kind in ("Assign", "Let") and m.name not in names and m.name not in exclude:
                names.append(m.name)
            if m.kind == "For" and m.var not in names and m.var not in exclude:
                names.append(m.var)
        lowered = [n.lower() for n in names]
        if len(set(lowered)) != len(lowered):
            raise Skip("identifiers that differ only in case")
        return names

    def pascal_function(self, s, scope, depth, param_kinds, ret_kind):
        """`function f(a: integer): integer;` with its var section and body;
        the tail return assigns the function's name, other returns exit
        with their value; a function with no result is a procedure."""
        ind = self.indent(depth)
        name = self.ident(s.name, True)
        inner = FunctionScope(scope, s)
        params = "; ".join(f"{self.ident(p)}: {self.type_word(k)}" for (p, _), k in zip(s.params, param_kinds))
        body = self.inline.get(s.name, []) + explicit_returns(s.body)
        if ret_kind == NULL:
            header = f"procedure {name}({params});"
        else:
            header = f"function {name}({params}): {self.type_word(ret_kind)};"
            if body and body[-1].kind == "Return" and body[-1].expr is not None:
                body[-1] = Node("Assign", name=s.name, expr=body[-1].expr, line=body[-1].line)
        env = dict(scope.kinds_env)
        for (p, _), k in zip(s.params, param_kinds):
            env[p] = k
        self.kinds.infer_body(body, env)
        lines = [ind + header]
        exclude = {p for p, _ in s.params} | {s.name}
        for n in self.pascal_names(body, exclude):
            lines.append(ind + f"var {self.ident(n)}: {self.type_word(env.get(n, UNKNOWN))};")
            inner.declare(n)
        inner.declare(s.name)
        lines.append(ind + "begin")
        lines.extend(self.statements(body, inner, depth + 1))
        lines.append(ind + "end;")
        return lines


class PostfixEmitter(Emitter):
    """A postfix target (RPLumen): expressions in post-order, one value per
    call, control words around bodies, quoted names for the words that
    take one. Precedence, declarations and call brackets do not arise."""

    # Builtins that consume their arguments and yield nothing.
    CONSUMERS = ("print", "write", "emit", "error", "push")

    def check_globals(self, prog, lib_globals, lib_fns):
        # Bindings are looked up through every open frame, as in Lumen.
        self.inline = {}

    def program(self, prog, lib_globals, lib_fns, source_path):
        scope = Scope(prog.body, lib_fns)
        self.check_globals(prog, lib_globals, lib_fns)
        lines = [f"{self.w('lexical.comment_line')} {HEADER.format(source=source_path)}"]
        for s in lib_globals + lib_fns + prog.body:
            lines.extend(self.statement(s, scope, 0))
            if s.kind == "Fn":
                lines.append("")
        return "\n".join(lines).rstrip("\n") + "\n"

    def quoted(self, name):
        q = self.w("lexical.name_quote")
        return f"{q}{self.ident(name)}{q}"

    # ---- expressions: the text leaves one value on the stack
    def expr(self, e, scope):
        if e.kind == "Num":
            return self.number(e), None
        if e.kind == "Str":
            return self.string(e.value), None
        if e.kind == "Bool":
            return self.w("literal.true" if e.value else "literal.false"), None
        if e.kind == "Null":
            return self.w("literal.null"), None
        if e.kind == "Var":
            if e.name in SYSTEM_NAMES:
                raise Skip(f"no `{e.name}`")
            return self.ident(e.name), None
        if e.kind == "Group":
            return self.expr(e.expr, scope)
        if e.kind == "Array":
            items = " ".join(self.expr(i, scope)[0] for i in e.items)
            return f"{self.w('syntax.array.open', 'array literal')} {items} {self.w('syntax.array.close')}".replace("  ", " "), None
        if e.kind == "Index":
            target, _ = self.expr(e.target, scope)
            index, _ = self.expr(e.index, scope)
            return f"{target} {index} {self.w('builtin.get', 'indexing')}", None
        if e.kind == "Extern":
            raise Skip("no `extern`")
        if e.kind == "Unary":
            text, _ = self.expr(e.operand, scope)
            return f"{text} {self.w('op.negate' if e.op == '-' else 'op.not')}", None
        if e.kind == "Bin":
            left, _ = self.expr(e.left, scope)
            right, _ = self.expr(e.right, scope)
            if e.op in ("and", "or"):
                # Short-circuit as a branch: the right side runs only when
                # the left side leaves the answer open.
                if_, else_, end = self.w("stmt.if"), self.w("stmt.else"), self.w("block.close")
                if e.op == "and":
                    return f"{left} {if_} {right} {else_} {self.w('literal.false')} {end}", None
                return f"{left} {if_} {self.w('literal.true')} {else_} {right} {end}", None
            op = e.op
            if op == "." and not self.has("op.concat"):
                op = "+"
            return f"{left} {right} {self.w(self.OP_LABELS[op], e.op)}", None
        if e.kind == "Call":
            return self.call(e, scope), None
        raise Skip(f"unexpected {e.kind}")

    def call(self, e, scope):
        name = e.name
        args = e.args
        if name in POLYMORPHIC and self.has(POLYMORPHIC[name]):
            return self.postfix_call(self.d[POLYMORPHIC[name]][0], args, scope)
        if name in scope.functions:
            if len(args) != len(scope.functions[name].params):
                raise Skip(f"`{name}` called with the wrong number of arguments")
            return self.postfix_call(self.ident(name, True), args, scope)
        if name in ("print", "write"):
            if len(args) != 1:
                raise Skip("print with several arguments")
            return self.postfix_call(self.w(f"builtin.{name}"), args, scope)
        if name == "push":
            # push(arr, value): the array is named, not evaluated.
            if len(args) != 2 or args[0].kind != "Var":
                raise Skip("push to an expression")
            value, _ = self.expr(args[1], scope)
            return f"{value} {self.quoted(args[0].name)} {self.w('builtin.push')}"
        if name == "real" and len(args) == 1:
            # A real takes its precision from the stack: the default is 15.
            args = args + [Node("Num", value=Fraction(15), real=False, base=10, text="15")]
        if name in KERNEL_BUILTINS:
            return self.postfix_call(self.w(KERNEL_BUILTINS[name], name), args, scope)
        raise Skip(f"unknown function `{name}`")

    def postfix_call(self, word, args, scope):
        parts = [self.expr(a, scope)[0] for a in args]
        return " ".join(parts + [word])

    # ---- statements
    def statement(self, s, scope, depth, is_entry=False):
        ind = self.indent(depth)
        k = s.kind
        if k == "Expr":
            e = s.expr
            text, _ = self.expr(e, scope)
            consumed = e.kind == "Call" and e.name in self.CONSUMERS
            return [ind + text + ("" if consumed else f" {self.w('stack.drop')}")]
        if k in ("Assign", "Let"):
            value = self.w("literal.null") if s.expr is None else self.expr(s.expr, scope)[0]
            return [ind + f"{value} {self.quoted(s.name)} {self.w('stmt.assign')}"]
        if k == "IndexAssign":
            if s.target.kind != "Var":
                raise Skip("indexed assignment to an expression")
            index, _ = self.expr(s.index, scope)
            value, _ = self.expr(s.expr, scope)
            return [ind + f"{index} {value} {self.quoted(s.target.name)} {self.w('builtin.put', 'indexed assignment')}"]
        if k == "Memo":
            raise Skip("no memoization switch")
        if k == "Return":
            # Every function leaves exactly one value; a bare return leaves null.
            value = self.w("literal.null") if s.expr is None else self.expr(s.expr, scope)[0]
            return [ind + f"{value} {self.w('stmt.return')}"]
        if k == "Break":
            return [ind + self.w("stmt.break")]
        if k == "Continue":
            return [ind + self.w("stmt.continue")]
        if k == "If":
            cond, _ = self.expr(s.cond, scope)
            lines = [ind + f"{cond} {self.w('stmt.if')}"] + self.statements(s.body, scope, depth + 1)
            if s.orelse:
                lines.append(ind + self.w("stmt.else"))
                lines.extend(self.statements(s.orelse, scope, depth + 1))
            return lines + [ind + self.w("block.close")]
        if k == "While":
            cond, _ = self.expr(s.cond, scope)
            head = f"{self.w('stmt.while')} {cond} {self.d['block.intro'][0]}"
            return [ind + head] + self.statements(s.body, scope, depth + 1) + [ind + self.w("block.close")]
        if k == "Until":
            cond, _ = self.expr(s.cond, scope)
            body = self.statements(s.body, scope, depth + 1)
            return [ind + self.w("stmt.until")] + body + [ind + f"{self.d['block.intro'][1]} {cond} {self.w('block.close')}"]
        if k == "For":
            start, _ = self.expr(s.start, scope)
            end, _ = self.expr(s.end, scope)
            head = f"{start} {end} {self.quoted(s.var)} {self.w('stmt.for')}"
            return [ind + head] + self.statements(s.body, scope, depth + 1) + [ind + self.d["block.close"][1]]
        if k == "Fn":
            return self.function(s, scope, depth)
        raise Skip(f"unexpected statement {k}")

    def function(self, s, scope, depth, is_entry=False):
        """`« 'b' = 'a' = body » 'name' =`: the parameters come off the
        stack last first, and the body leaves one value."""
        ind = self.indent(depth)
        inner = FunctionScope(scope, s)
        body = explicit_returns(s.body)
        if not body or body[-1].kind != "Return":
            body = body + [Node("Return", expr=None, line=s.line)]
        params = " ".join(f"{self.quoted(p)} {self.w('stmt.assign')}" for p, _ in reversed(s.params))
        open_, close = self.w("stack.program.open", "programs"), self.w("stack.program.close")
        lines = [ind + (f"{open_} {params}" if params else open_)]
        lines.extend(self.statements(body, inner, depth + 1))
        lines.append(ind + f"{close} {self.quoted(s.name)} {self.w('stmt.assign')}")
        return lines


def decimal_of(v):
    """A fraction whose denominator is 2^a 5^b as a decimal string."""
    sign = "-" if v < 0 else ""
    v = abs(v)
    whole = v.numerator // v.denominator
    rem = Fraction(v.numerator % v.denominator, v.denominator)
    digits = ""
    while rem:
        rem *= 10
        digits += str(rem.numerator // rem.denominator)
        rem = Fraction(rem.numerator % rem.denominator, rem.denominator)
    return f"{sign}{whole}.{digits or '0'}"


def step_before_continue(body, step):
    """The loop body with `step` before each `continue` that belongs to this
    loop rather than to a loop nested inside it."""
    out = []
    for s in body:
        if s.kind == "Continue":
            out.append(step)
            out.append(s)
        elif s.kind == "If":
            orelse = step_before_continue(s.orelse, step) if s.orelse else None
            out.append(Node("If", cond=s.cond, body=step_before_continue(s.body, step), orelse=orelse, line=s.line))
        else:
            out.append(s)
    return out


def loop_body_nodes(body):
    """Nodes of a loop body outside any nested loop."""
    for s in body:
        yield s
        if s.kind == "If":
            yield from loop_body_nodes(s.body)
            if s.orelse:
                yield from loop_body_nodes(s.orelse)


def explicit_returns(body):
    """A function body whose tail expression is written as a return."""
    if not body:
        return body
    body = list(body)
    last = body[-1]
    if last.kind == "Expr" and not (last.expr.kind == "Call" and last.expr.name in ("print", "write")):
        body[-1] = Node("Return", expr=last.expr, line=last.line)
    elif last.kind == "If":
        body[-1] = Node("If", cond=last.cond, body=explicit_returns(last.body),
                        orelse=explicit_returns(last.orelse) if last.orelse else None, line=last.line)
    return body


def hoisted_names(body):
    """Names first assigned inside a nested block: declared at the top of
    the scope in languages whose declarations are block-scoped."""
    top = set()
    nested = []
    for s in body:
        if s.kind in ("Assign", "Let"):
            top.add(s.name)
        elif s.kind in ("If", "While", "Until", "For"):
            for m in walk(s):
                if m.kind in ("Assign", "Let") and m.name not in top and m.name not in nested:
                    nested.append(m.name)
            if s.kind == "For":
                top.add(s.var)
    return [n for n in nested if n not in top]


class Scope:
    """Declared names of the program level, and every function in reach."""

    def __init__(self, body, lib_fns):
        self.declared = set()
        self.functions = {s.name: s for s in body + lib_fns if s.kind == "Fn"}
        self.counts = assignment_counts(body)
        self.kinds_env = {}
        self.block_depth = 0

    def is_declared(self, name):
        return name in self.declared

    def declare(self, name):
        self.declared.add(name)

    def reassigned(self, name):
        return self.counts.get(name, 0) > 1

    def declaring(self, body):
        return _Depth(self)


class FunctionScope(Scope):
    def __init__(self, outer, fn):
        self.declared = {p for p, _ in fn.params}
        self.functions = outer.functions
        self.counts = assignment_counts(fn.body)
        self.kinds_env = dict(outer.kinds_env)
        self.block_depth = 0


class _Depth:
    def __init__(self, scope):
        self.scope = scope

    def __enter__(self):
        self.scope.block_depth += 1

    def __exit__(self, *a):
        self.scope.block_depth -= 1


def assignment_counts(body):
    counts = {}
    for s in walk(Node("B", body=body)):
        if s.kind in ("Assign", "Let", "IndexAssign"):
            name = s.name if s.kind != "IndexAssign" else None
            if name:
                counts[name] = counts.get(name, 0) + 1
        elif s.kind == "For":
            counts[s.var] = counts.get(s.var, 0) + 2
    return counts


# ---------------------------------------------------------------- driver

def definitions():
    defs = {}
    for path in sorted(LANGS.glob("*.json")) + sorted((LANGS / "extras").glob("*.json")):
        d = json.loads(path.read_text(encoding="utf-8"))
        if d["language"] != "lumen":
            defs[d["language"]] = d
    return defs


def port_one(emitter, source_path, lib, constants):
    try:
        prog = parse(source_path.read_text(encoding="utf-8"))
    except SyntaxError as e:
        raise SyntaxError(f"{source_path}: {e}") from None
    replaced = {name for name, label in POLYMORPHIC.items() if emitter.has(label)}
    lib_globals, lib_fns = library_closure(prog, lib, constants, replaced)
    rel = source_path.relative_to(ROOT)
    return emitter.program(prog, lib_globals, lib_fns, str(rel))


def main():
    lib, constants = load_library()
    defs = definitions()
    examples = sorted(LUMEN_EXAMPLES.rglob("*.lm"))
    results = {}  # (example, language) -> None or reason
    written = {name: 0 for name in defs}
    for lang, d in defs.items():
        ext = d["extensions"][0]
        out_root = ROOT / "examples" / lang
        # Remove earlier ports so a newly skipped example leaves no stale file.
        for old in out_root.rglob(f"*.{ext}"):
            if old.read_text(encoding="utf-8", errors="replace").find("port_examples.py") >= 0:
                old.unlink()
        for ex in examples:
            rel = ex.relative_to(LUMEN_EXAMPLES).with_suffix(f".{ext}")
            try:
                emitter = PostfixEmitter(d) if d["block.style"] == "postfix" else Emitter(d)
                text = port_one(emitter, ex, lib, constants)
            except Skip as why:
                results[(ex, lang)] = str(why)
                continue
            target = out_root / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8")
            results[(ex, lang)] = None
            written[lang] += 1
    write_report(examples, defs, results, written)
    total = sum(written.values())
    print(f"ported {total} programs: " + ", ".join(f"{k} {v}" for k, v in written.items()))
    return 0


def write_report(examples, defs, results, written):
    langs = list(defs)
    lines = ["# Lumen examples in every language", "",
             "Generated by `scripts/port_examples.py` from `examples/lumen/`. A cell says",
             "`yes` when the example is written in that language under `examples/<language>/`,",
             "in the same relative path, or names the first construct the language's",
             "definition has no spelling for. The library functions an example uses are",
             "ported into the file with it.", "",
             f"Lumen has {len(examples)} examples; " + ", ".join(f"{l} carries {written[l]}" for l in langs) + ".", "",
             "| Example | " + " | ".join(langs) + " |",
             "|---|" + "---|" * len(langs)]
    for ex in examples:
        rel = ex.relative_to(LUMEN_EXAMPLES)
        cells = []
        for l in langs:
            why = results[(ex, l)]
            cells.append("yes" if why is None else why)
        lines.append(f"| `{rel}` | " + " | ".join(cells) + " |")
    REPORT.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    sys.exit(main())
