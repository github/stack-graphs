class Foo {
    bar() {

    }
}

class Baz extends Foo { }

let obj = new Baz();
obj.bar;
//  ^ defined: 2