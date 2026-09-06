// Ported from examples/lumen/constructs/string_mixed.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var message: string;
var n: integer;
begin
    message := 'value: ';
    n := 42;
    writeln(message);
    writeln(n);
    if n = 42 then begin
        writeln('correct');
    end;
end.
