class Foo {
    bar = 1;
    baz() { }
    quux() {
        this.bar;
        //   ^ defined: 2

        this.baz();
        //   ^ defined: 2
    }
}