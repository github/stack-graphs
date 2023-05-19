class Foo
  def bar
  end

  def self.bar
  end
end

  Foo.new.bar
# ^ defined: 1
        # ^ defined: 2

  Foo.bar
#     ^ defined: 5

