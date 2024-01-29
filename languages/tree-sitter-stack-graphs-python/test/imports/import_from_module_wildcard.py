# --- path: foo.py ---

FOO = 42

# --- path: test.py ---

from foo import *

FOO
# ^ defined: 3

foo
# ^ defined:
