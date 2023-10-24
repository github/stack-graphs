let obj = {
    x: 1
};



class Foo {
    // method declaration
    meth_1() {
        return obj;
    }

    // generator method declaration
    * gen_meth_1() {
        yield obj;
    }
}

let foo = new Foo();

foo.meth_1().x;
//           ^ defined: 2

foo.gen_meth_1().x;
//               ^ defined: 2