let x = {
    f: {
        v: 42
    }
}

with(x) {
    f.v;
//  ^ defined: 2
//    ^ defined: 3
}

export {};
