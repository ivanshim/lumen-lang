// Ported from examples/lumen/constructs/scope_leak.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var y: integer;

begin
    y := 100;
    writeln(y);
    if true then begin
        y := 50;
        writeln(y);
    end;
    writeln(y);
end.
