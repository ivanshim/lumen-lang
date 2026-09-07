// The Lumen library file lib_lumen/number_theory.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function gcd(a: integer; b: integer): integer;
var t: integer;
begin
    while b <> 0 do begin
        t := b;
        b := a mod b;
        a := t;
    end;
    gcd := a;
end;

function lcm(a: integer; b: integer): integer;
begin
    lcm := (a * b) div gcd(a, b);
end;

function is_coprime(a: integer; b: integer): boolean;
begin
    is_coprime := gcd(a, b) = 1;
end;

function is_prime(n: integer): boolean;
var i: integer;
begin
    if n < 2 then begin
        exit(false);
    end;
    if n = 2 then begin
        exit(true);
    end;
    if n mod 2 = 0 then begin
        exit(false);
    end;
    i := 3;
    while i * i <= n do begin
        if n mod i = 0 then begin
            exit(false);
        end;
        i := i + 2;
    end;
    is_prime := true;
end;

function isqrt(n: integer): integer;
var x: integer;
var x1: integer;
begin
    if n = 0 then begin
        exit(0);
    end;
    x := n;
    while true do begin
        x1 := (x + n div x) div 2;
        if x1 >= x then begin
            exit(x);
        end;
        x := x1;
    end;
end;

function is_unit(a: integer; m: integer): boolean;
begin
    is_unit := gcd(a mod m, m) = 1;
end;
