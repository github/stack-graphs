#------ path: one/two.py -----------#

# module
import a.b.c

d = 1

e = a.b

#------ path: three/__init__.py ---#

# module
f = 3

#------ path: main.py -------------#

from one.two import d, e.c
#        ^ defined: 3
#                   ^ defined: 6
#                      ^ defined: 4, 8

import three
#      ^ defined: 12

print(d, e.c)
#     ^ defined: 6, 17
#          ^ defined: 4, 17

print three.f
#     ^ defined: 12, 22
#           ^ defined: 13
