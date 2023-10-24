let obj = {
    x: 1
};



class Foo {
    // method declaration
    meth_1() {
        return arguments;
    }

    // generator method declaration
    * gen_meth_1() {
        yield arguments;
    }
}

let foo = new Foo();

foo.meth_1(obj)[0].x;
//                 ^ defined: 2

foo.gen_meth_1(obj)[0].x;
//                     ^ defined: 2