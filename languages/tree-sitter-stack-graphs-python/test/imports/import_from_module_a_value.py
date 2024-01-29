# --- path: foo.py ---

FOO = 42

# --- path: test.py ---

from foo import FOO

FOO
# ^ defined: 7, 3

foo
# ^ defined:
