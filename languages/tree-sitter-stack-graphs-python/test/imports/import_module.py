# --- path: foo.py ---

FOO = 42

# --- path: test.py ---

import foo

foo.FOO
# ^ defined: 7, 3
#     ^ defined: 3
