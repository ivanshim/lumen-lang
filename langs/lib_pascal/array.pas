// The Lumen library file langs/lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function array_index_of(a: integer; x: integer): integer;
var i: integer;
begin
    i := 0;
    while i < length(a) do begin
        if a[i] = x then begin
            exit(i);
        end;
        i := i + 1;
    end;
    array_index_of := -1;
end;

function array_contains(a: integer; x: integer): boolean;
begin
    array_contains := array_index_of(a, x) >= 0;
end;
