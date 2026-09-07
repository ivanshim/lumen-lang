// The Lumen library file lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function repeat_string(s: integer; repetitions: integer): string;
var out: string;
var i: integer;
begin
    out := '';
    i := 0;
    while i < repetitions do begin
        out := out + s;
        i := i + 1;
    end;
    repeat_string := out;
end;

function join_strings(arr: integer; separator: integer): string;
var out: string;
var n: integer;
var i: integer;
begin
    out := '';
    n := length(arr);
    i := 0;
    while i < n do begin
        if i > 0 then begin
            out := out + separator;
        end;
        out := out + arr[i];
        i := i + 1;
    end;
    join_strings := out;
end;
