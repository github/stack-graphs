let bar = 42;

class Foo {
    static foo: number;
    static {
        this.foo = bar;
        //   ^ defined: 4
        //         ^ defined: 1
    }
}

export {}
