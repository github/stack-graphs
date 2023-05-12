module A
  CONST = 1
  def a; end
end

module B
  include A

  def b; end
end

include B

C = Module.new do
  def c; end
  def b
    puts "c"
  end
end

self.extend(C)

class D
  include C

  def calling_a
    puts "calling a: #{a}"
  end

  def calling_c
    puts "calling c: #{c}"
  end
end

d = D.new
d.b # => "c"
