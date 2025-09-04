y = x = 1

no_params = lambda: y
            #       ^ defined: 1

sorted([1, 2, 3], key=lambda y: x + y)
                      #         ^ defined: 1

def uses_default(fn=lambda: 1):
    fn()
#   ^ defined: 9
