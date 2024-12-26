class Foo
end

class Foo::A
end

  Foo::A
# ^ defined: 1,4
#      ^ defined: 4

  ::Foo
#   ^ defined: 1, 4
