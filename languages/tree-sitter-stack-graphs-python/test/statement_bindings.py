import a.b
import c.d

with a as x, c as y:
    print x.b, y.d
    #     ^ defined: 1, 4
    #       ^ defined: 1
    #          ^ defined: 2, 4
    #            ^ defined: 2
