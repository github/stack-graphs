a = 1

a.b = 2

a.c.d = 5

print a.b
#     ^ defined: 1
#       ^ defined: 3

print a.c, a.c.d
#     ^ defined: 1
#       ^ defined:
#              ^ defined: 5
