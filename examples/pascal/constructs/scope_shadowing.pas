// Ported from examples/lumen/constructs/scope_shadowing.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: integer;
begin
    writeln(1);
    x := 10;
    writeln(x);
    if true then begin
        x := 20;
        writeln(x);
    end;
    writeln(x);
end.
