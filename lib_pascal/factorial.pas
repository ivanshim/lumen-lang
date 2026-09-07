// The Lumen library file lib_lumen/factorial.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function factorial(n: integer): integer;
begin
    if n <= 1 then begin
        exit(1);
    end else begin
        exit(n * factorial(n - 1));
    end;
end;
