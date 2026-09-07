// The Lumen library file langs/lib_lumen/render.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function bool_to_string(b: integer): string;
begin
    if b then begin
        exit('true');
    end;
    bool_to_string := 'false';
end;

function null_to_string(x: integer): string;
begin
    null_to_string := 'null';
end;
