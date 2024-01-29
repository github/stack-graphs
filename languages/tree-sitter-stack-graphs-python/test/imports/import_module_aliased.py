# --- path: foo.py ---

FOO = 42

# --- path: test.py ---

import foo as qux

qux.FOO
# ^ defined: 7, 3
#     ^ defined: 3

foo
# ^ defined:
