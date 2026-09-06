// Ported from examples/lumen/constructs/loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var x: integer;
begin
    x := 0;
    while x < 10 do begin
        write(x);
        if x < 9 then begin
            write(', ');
        end;
        x := x + 1;
    end;
    writeln('');
end.
