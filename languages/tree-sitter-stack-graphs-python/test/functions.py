import a.x.y
import b.x.y

def get_x(value):
    return value.x
    #      ^ defined: 4

print get_x(a).y
#              ^ defined: 1

print get_x(b).y
#              ^ defined: 2

def get_a():
    return a

print get_a(b).x
#              ^ defined: 1

print get_x(foo=1, value=a).y
#                           ^ defined: 1

def foo(w: int, x, y=1, z: int=4, *args, **dict):
    local = x
#           ^ defined: 23
    print(args, w, z)
#         ^ defined: 23
#               ^ defined: 23
#                  ^ defined: 23
    return y
#          ^ defined: 23
