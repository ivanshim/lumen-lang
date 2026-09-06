// Ported from examples/lumen/constructs/for_loop_continue.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var i: integer;

begin
    i := 0;
    while i < 11 do begin
        if i = 5 then begin
            i := i + 1;
            continue;
        end;
        write(i);
        if i < 10 then begin
            write(', ');
        end;
        i := i + 1;
    end;
    writeln('');
end.
