// The Lumen library file lib_lumen/string_ord_chr.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function is_ascii(c: integer): boolean;
begin
    is_ascii := ord(c) < 128;
end;

function is_digit(c: integer): boolean;
var o: integer;
begin
    o := ord(c);
    is_digit := (o >= ord('0')) and (o <= ord('9'));
end;

function is_alpha(c: integer): boolean;
var o: integer;
begin
    o := ord(c);
    is_alpha := ((o >= ord('A')) and (o <= ord('Z'))) or ((o >= ord('a')) and (o <= ord('z')));
end;

function is_alnum(c: integer): boolean;
begin
    is_alnum := is_alpha(c) or is_digit(c);
end;

function char_to_upper(c: string): string;
var o: integer;
begin
    o := ord(c);
    if (o >= ord('a')) and (o <= ord('z')) then begin
        exit(chr(o - 32));
    end else begin
        exit(c);
    end;
end;

function char_to_lower(c: string): string;
var o: integer;
begin
    o := ord(c);
    if (o >= ord('A')) and (o <= ord('Z')) then begin
        exit(chr(o + 32));
    end else begin
        exit(c);
    end;
end;

function is_whitespace(c: integer): boolean;
var o: integer;
begin
    o := ord(c);
    is_whitespace := (o = 32) or (o = 9) or (o = 10) or (o = 13);
end;
