#--- path: a.py ---#

def f0(a): return X.a + X.b
def f1(a): return f0(1)
def f2(a): return f1(2)
def f3(a): return f2(3)
def f4(a): return f3(4)
def f5(a): return f4(5)
def f6(a): return f5(6)
def f7(a): return f6(7)
def f8(a): return f7(8)
def f9(a): return f8(9)

class C1:
    def m0(self, b): return f9(0)
    def m1(self, b): return self.m0(1)
    def m2(self, b): return self.m1(2)
    def m3(self, b): return self.m2(3)
    def m4(self, b): return self.m3(4)
    def m5(self, b): return self.m4(5)
    def m6(self, b): return self.m5(6)
    def m7(self, b): return self.m6(7)
    def m8(self, b): return self.m7(8)

def f10(): return C1.m8(0)
def f11(): return f10(1)
def f12(): return X.c(2)
def f13(): return X.c(3)
def f14(): return x(4)
def f15(): return x(5)
def f16(): return x(6)
def f17(): return x(7)
def f18(): return x(8)

class C2:
    def m0(self): return X.d(0)
    def m1(self): return X.d(1)
    def m2(self): return X.d(2)
    def m3(self): return X.d(3)
    def m4(self): return X.d(4)
    def m5(self): return X.d(5)
    def m6(self): return X.d(6)
    def m7(self): return X.d(7)
    def m8(self): return X.d(8)

#--- path: main.py ---#

from a import *

print f0(), f4(), f8()
#     ^ defined: 3
#           ^ defined: 7
#                 ^ defined: 11

print C1.m0, C1().m0(), C1.m4, C1().m4, C1.m8, C1.m8
#     ^ defined: 14
#        ^ defined: 15
#                 ^ defined: 15
#                          ^ defined: 19
#                                   ^ defined: 19
#                                          ^ defined: 23
#                                                 ^ defined: 23

print f10(), f14(), f18()
#     ^ defined: 25
#             ^ defined: 29
#                   ^ defined: 33

print C2.m0, C2().m0(), C2.m4, C2().m4, C2.m8, C2.m8
#     ^ defined: 35
#        ^ defined: 36
#                 ^ defined: 36
#                          ^ defined: 40
#                                   ^ defined: 40
#                                          ^ defined: 44
#                                                 ^ defined: 44
