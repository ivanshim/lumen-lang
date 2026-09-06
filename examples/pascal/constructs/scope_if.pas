// Ported from examples/lumen/constructs/scope_if.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: integer;
begin
    x := 10;
    if true then begin
        x := 20;
        writeln(x);
    end else begin
        x := 30;
        writeln(x);
    end;
    writeln(x);
end.
