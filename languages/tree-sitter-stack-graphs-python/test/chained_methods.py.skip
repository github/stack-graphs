class Builder:
    def set_a(self, a):
        self.a = a
        return self

    def set_b(self, b):
        self.b = b
        return self

    def set_c(self, c):
        self.c = c
        return self

    def set_d(self, d):
        self.d = d
        return self

    def set_e(self, e):
        self.d = d
        return self

Builder().set_a('a1').set_b('b2').set_c('c3').set_d('d4').set_e('e4')
#         ^ defined: 2
#                     ^ defined: 6
#                                 ^ defined: 10
#                                             ^ defined: 14
#                                                         ^ defined: 18
