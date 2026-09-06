// Ported from examples/lumen/constructs/scope_nested.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: integer;

begin
    x := 1;
    if true then begin
        x := 2;
        if true then begin
            x := 3;
            writeln(x);
        end;
        writeln(x);
    end;
    writeln(x);
end.
