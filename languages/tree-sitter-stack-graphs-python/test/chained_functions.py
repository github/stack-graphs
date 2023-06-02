class A:
    w = 1
    x = 2
    y = 3
    z = 4

def get_a():
    return A
def get_b():
    return get_a()
def get_c():
    return get_b()
def get_d():
    return get_c()
def get_e():
    return get_d()
def get_f():
    return get_e()

g = get_f(A)
print g.x, g.y
#       ^ defined: 3
#            ^ defined: 4
