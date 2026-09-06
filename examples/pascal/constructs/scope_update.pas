// Ported from examples/lumen/constructs/scope_update.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var counter: integer;

begin
    counter := 0;
    if true then begin
        counter := counter + 1;
    end;
    if true then begin
        counter := counter + 1;
    end;
    writeln(counter);
end.
