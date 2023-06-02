#---- path: a.py -------

class A:
    def __init__(self, b, c):
        self.b = b
        self.c = A(c, 1)

    def get_b(self):
        return self.b
        #           ^ defined: 4, 5

    def get_c(self):
        return self.c
        #           ^ defined: 6

    def get_all(self):
        return [self.get_b(), self.get_c()]
        #            ^ defined: 8
        #                          ^ defined: 12

a = A(1, 2)
a.get_all()
# ^ defined: 16

a.b
# ^ defined: 4, 5

a.c.b
#   ^ defined: 4, 5
# ^ defined: 6

#----- path: main.py ---------

import a

print a.A, a.a
#       ^ defined: 3
#            ^ defined: 21
