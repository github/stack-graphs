def f():
    if a:
        b = 1
    else:
        c = 2

    print b, c
    #     ^ defined: 3
    #        ^ defined: 5

class G:
    if d:
        e = 1

    print e
    #     ^ defined: 13

print b, c, e
#     ^ defined:
#        ^ defined:
#           ^ defined:
