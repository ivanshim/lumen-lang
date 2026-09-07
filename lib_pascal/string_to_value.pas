// The Lumen library file lib_lumen/string_to_value.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function character_to_value(c: string): integer;
var code: integer;
begin
    if is_digit(c) then begin
        exit(ord(c) - ord('0'));
    end;
    code := ord(c);
    if (code >= ord('A')) and (code <= ord('Z')) then begin
        exit(code - ord('A') + 10);
    end;
    if (code >= ord('a')) and (code <= ord('z')) then begin
        exit(code - ord('a') + 10);
    end;
    character_to_value := -1;
end;
