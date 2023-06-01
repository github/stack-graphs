#------ path: foo.py ------#

# module
class A:
    a = 1

class B:
    class C:
        class D:
            d = 2

#------ path: main.py ---#

from foo import A as X, B.C.D as Y
import foo as f

print X.a, Y.d
#       ^ defined: 5
#            ^ defined: 10

print A, B.C
#     ^ defined:
#        ^ defined:
#          ^ defined:

print f.B
#     ^ defined: 2, 15
#       ^ defined: 7

print foo
#     ^ defined:
