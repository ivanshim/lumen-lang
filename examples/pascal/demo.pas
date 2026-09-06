{ A demo: declarations first, then the main block. }
var x: integer;
var y: integer;
var i: integer;
var sum: integer;
begin
  writeln(1 + 2 * 3);
  x := 0;
  y := 5;

  if (x < y) and (y = 5) then begin
    writeln(100);
  end else begin
    writeln(200);
  end;

  i := 0;
  sum := 0;
  while i < 10 do begin
    if i = 5 then begin
      i := i + 1;
      continue;
    end;
    if i = 8 then begin
      break;
    end;
    sum := sum + i;
    writeln(sum);
    i := i + 1;
  end;

  writeln(sum);
  writeln(true);
  writeln(not false);
  writeln(-10 + 3);
  writeln(17 div 5);
  writeln(17 mod 5);
  writeln(x <> y);
  WriteLn('Keywords and identifiers ignore case');
  (* Another comment style. *)
end.
