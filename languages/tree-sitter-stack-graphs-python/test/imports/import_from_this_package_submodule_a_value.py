# --- path: foo/bar.py ---

BAR = 42

# --- path: foo/test.py ---

from .bar import BAR

BAR
# ^ defined: 7, 3
