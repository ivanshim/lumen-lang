def fib(n)
  a = 0
  b = 1
  for i in 0...n
    c = a + b
    a = b
    b = c
  end
  return a
end

for i in 0...10
  puts(fib(i))
end
