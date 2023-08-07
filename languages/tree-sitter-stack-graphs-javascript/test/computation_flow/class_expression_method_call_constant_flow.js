let Foo = class {
    // method declaration
    meth_1() {
        return {
            x: 1
        };
    }

    // generator method declaration
    * gen_meth_1() {
        yield {
            x: 1
        };
    }
};

let foo = new Foo();

foo.meth_1().x;
//           ^ defined: 5

foo.gen_meth_1().x;
//               ^ defined: 12