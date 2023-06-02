a = 1

class B:
    c = a
    #   ^ defined: 1

    def d(self):
        return self.c

    class E:
        f = a
        #   ^ defined: 1

print B.c
#     ^ defined: 3
#       ^ defined: 1, 4

print B.d(1)
#       ^ defined: 7

print B.a, E.a
#       ^ defined:
#            ^ defined:
