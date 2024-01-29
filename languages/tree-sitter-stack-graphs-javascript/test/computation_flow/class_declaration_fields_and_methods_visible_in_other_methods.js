class Foo {
    bar = 1;
    baz() { }
    quux() {
        this.bar;
        //   ^ defined: 2

        this.baz();
        //   ^ defined: 3
    }
}

(class {
    bar = 1;
    baz() { }
    quux() {
        this.bar;
        //   ^ defined: 14

        this.baz();
        //   ^ defined: 15
    }
});