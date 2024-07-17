class C
end

  C.new.foo
# ^ defined: 1
#       ^ defined:

module M
    refine C do
        def foo
        end
    end
end

  C.new.foo
# ^ defined: 1
#       ^ defined:

using M

  C.new.foo
# ^ defined: 1
#       ^ defined: 10
