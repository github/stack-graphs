class a:
    x = 1
class b:
    x = 1
class c:
    x = 1

(a1, b1) = (a, b)
a2, b2 = (a, b)
(a3, b3) = (a, b)
a4, b4 = a, b

print a1.x, b1.x
#        ^ defined: 2
#              ^ defined: 4
print a2.x, b2.x
#        ^ defined: 2
#              ^ defined: 4
print a3.x, b3.x
#        ^ defined: 2
#              ^ defined: 4
print a4.x, b4.x
#        ^ defined: 2
#              ^ defined: 4

t = (a, b), c
(a5, b5), c5 = t

print a5.x, b5.x, c5.x
#        ^ defined: 2
#              ^ defined: 4
#                    ^ defined: 6

(a6, (b6, c6)) = (a, (b, c))

print a6.x, b6.x
#        ^ defined: 2
#              ^ defined: 4
