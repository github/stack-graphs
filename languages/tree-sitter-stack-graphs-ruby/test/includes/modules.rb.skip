# --- path: a_module.rb ---
module A
  CONST = 1
  def a; end
end

# --- path: b_module.rb ---
$LOAD_PATH << '.'
require 'a_module'

module B
  include A
        # ^ defined: 1

  def b; end
end

include B
      # ^ defined: 4
  a
# ^ defined: 3

  CONST
# ^ defined: 2

C = Module.new do
  def c; end
  def b; end
end

self.extend(C)
          # ^ defined: 19

self.c
   # ^ defined: 20

# --- path: d_class.rb ---
$LOAD_PATH << '.'
require 'b_module'

class D
  include B
        # ^ defined: 4
  extend C
       # ^ defined: 19

  def calling_a
    puts "calling a: #{a}"
                     # ^ defined: 3
  end

  def calling_b
    puts "calling b: #{b}"
                     # ^ defined: 8
  end

  def self.calling_b
    puts "calling self.b: #{b}"
                          # ^ defined: 21
  end

  def self.calling_c
    puts "calling c: #{c}"
                     # ^ defined: 20
  end
end

d = D.new
  # ^ defined: 4
d.c
# ^ defined: 5, 20
d.calling_c
# ^ defined: 13
d.b
# ^ defined: 4, 21
