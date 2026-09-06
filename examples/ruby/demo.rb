puts(1 + 2 * 3)

x = 0
y = 5

if x < y && y == 5
  puts(100)
elsif x == y
  puts(150)
else
  puts(200)
end

i = 0
sum = 0

while i < 10
  if i == 5
    i = i + 1
    next
  end

  if i == 8
    break
  end

  sum = sum + i
  puts(sum)
  i = i + 1
end

puts(sum)
puts(true)
puts(false)
puts(!false)
puts(-10 + 3)
puts(0x1F)
puts(2 ** 10)
=begin
A block comment.
=end
puts('single quotes keep \n literal')
