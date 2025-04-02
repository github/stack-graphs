sorted([1, 2, 3], key=lambda x: x)
#                               ^ defined: 1

y = 42

foo = lambda: print(y)
#                   ^ defined: 4

bar = lambda daz: print(y, daz)
#                       ^ defined: 4
#                          ^ defined: 9
