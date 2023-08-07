let obj = {
    x: 1
};



class Foo {
    // method declaration
    meth_1(o) {
        return o;
    }

    // generator method declaration
    * gen_meth_1(o) {
        yield o;
    }
}

let foo = new Foo();

foo.meth_1(obj).x;
//              ^ defined: 2

foo.gen_meth_1(obj).x;
//                  ^ defined: 2