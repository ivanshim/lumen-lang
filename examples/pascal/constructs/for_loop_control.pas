// Ported from examples/lumen/constructs/for_loop_control.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var i: integer;

begin
    i := 0;
    while i < 15 do begin
        if i = 10 then begin
            break;
        end;
        write(i);
        if i < 9 then begin
            write(', ');
        end;
        i := i + 1;
    end;
    writeln('');
end.
