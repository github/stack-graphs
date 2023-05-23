class C
end

module M
    class C
        def foo
        end
    end

      C.new.foo
    # ^ defined: 5
    #       ^ defined: 6
end

  M::C.new.foo
# ^ defined: 4
#    ^ defined: 5
#          ^ defined: 6

  C.new.foo
# ^ defined: 1
#       ^ defined:
