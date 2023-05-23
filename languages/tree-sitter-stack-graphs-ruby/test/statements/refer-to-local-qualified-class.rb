class N::C
end

module M
    class N::C
        def foo
        end
    end

      N::C.new.foo
    #    ^ defined: 5
    #          ^ defined: 6
end

  M::N::C.new.foo
# ^ defined: 4
#       ^ defined: 5
#             ^ defined: 6

  N::C.new.foo
#    ^ defined: 1
#          ^ defined:
