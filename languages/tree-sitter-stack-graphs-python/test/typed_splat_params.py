from typing import Any

def build(*args: Any, some: str | None = None, **kwargs: Any):
    print args
    #     ^ defined: 3
    print kwargs
    #     ^ defined: 3
    print some
    #     ^ defined: 3
