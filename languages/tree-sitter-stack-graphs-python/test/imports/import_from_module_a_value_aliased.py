# --- path: foo.py ---

FOO = 42

# --- path: test.py ---

from foo import FOO as QUX

QUX
# ^ defined: 7, 3

FOO
# ^ defined:
