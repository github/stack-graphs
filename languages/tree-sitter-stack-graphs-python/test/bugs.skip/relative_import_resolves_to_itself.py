#--- path: a/__init__.py ---#
from . import b
#             ^ defined: 6

#--- path: a/b/__init__.py ---#
B = 'b'

#--- path: main.py ---#
from a import b
#             ^ defined: 6

print b.B
#     ^ defined: 9, 6
#       ^ defined: 6