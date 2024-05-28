# ------ path: foo/bar/module.py -----------#
# ------ global: ROOT_PATH=foo/bar  -----------#

foo = 42

# ------ path: foo/bar/baz/module.py -----------#
# ------ global: ROOT_PATH=foo/bar  -----------#

bar = "hello"

# ------ path: foo/bar/main.py -------------#
# ------ global: ROOT_PATH=foo/bar  -----------#

from module import foo
from baz.module import bar

print(foo)
#     ^ defined: 4, 14

print(bar)
#     ^ defined: 9, 15
