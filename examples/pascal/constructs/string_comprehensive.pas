// Ported from examples/lumen/constructs/string_comprehensive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var a: string;
var b: string;
var x: integer;
var y: string;
begin
    a := 'alpha';
    b := 'beta';
    writeln(a);
    writeln(b);
    if a = 'alpha' then begin
        writeln('a is alpha');
    end;
    if a <> b then begin
        writeln('a and b are different');
    end;
    x := 10;
    y := 'number';
    writeln(x);
    writeln(y);
end.
