class A
  class B
    ::FOO = 42
  end
end

::FOO
# ^ defined: 3

::A::B::FOO
# ^ defined: 1
#    ^ defined: 2
#       ^ defined:
