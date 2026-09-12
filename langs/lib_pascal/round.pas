// The Lumen library file langs/lib_lumen/round.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function round(x: real; decimals: integer): integer;
var scale: integer;
var i: integer;
var y: real;
var r: integer;
var q: integer;
begin
    scale := 1;
    i := 0;
    while i < decimals do begin
        scale := scale * 10;
        i := i + 1;
    end;
    y := x * scale;
    if y >= 0 then begin
        r := (y * 2 + 1) div 2;
    end else begin
        r := (y * 2 - 1) div 2;
    end;
    q := r div scale;
    if q * scale = r then begin
        exit(q);
    end else begin
        exit(r / scale);
    end;
end;
