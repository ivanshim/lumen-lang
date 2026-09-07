// The Lumen library file langs/lib_lumen/modular_arithmetic.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function mod_mult(a: integer; b: integer; m: integer): integer;
begin
    mod_mult := (a * b) mod m;
end;

function mod_pow(base: integer; exp: integer; m: integer): integer;
var result: integer;
begin
    result := 1;
    base := base mod m;
    while exp > 0 do begin
        if exp mod 2 = 1 then begin
            result := (result * base) mod m;
        end;
        exp := exp div 2;
        base := (base * base) mod m;
    end;
    mod_pow := result;
end;
