// Ported from examples/lumen/constructs/scope_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var i: integer;
var sum: integer;
begin
    i := 0;
    sum := 0;
    while i < 5 do begin
        sum := sum + i;
        writeln(sum);
        i := i + 1;
    end;
    writeln(i);
    writeln(sum);
end.
