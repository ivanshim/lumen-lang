// Ported from examples/lumen/constructs/return_explicit.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function absolute(x: integer): integer;
begin
    if x < 0 then begin
        exit(-x);
    end;
    absolute := x;
end;

function safe_divide(a: integer; b: integer): integer;
begin
    if b = 0 then begin
        exit(nil);
    end;
    safe_divide := a / b;
end;

function find_first_even(a: integer; b: integer; c: integer): integer;
begin
    if a mod 2 = 0 then begin
        exit(a);
    end;
    if b mod 2 = 0 then begin
        exit(b);
    end;
    find_first_even := c;
end;

begin
    writeln('Test: Explicit Returns');
    writeln('absolute(-5):');
    writeln(absolute(-5));
    writeln(absolute(5));
    writeln('safe_divide(10, 2):');
    writeln(safe_divide(10, 2));
    writeln('safe_divide(10, 0):');
    writeln(safe_divide(10, 0));
    writeln('find_first_even(1, 2, 3):');
    writeln(find_first_even(1, 2, 3));
    writeln('find_first_even(2, 5, 7):');
    writeln(find_first_even(2, 5, 7));
end.
