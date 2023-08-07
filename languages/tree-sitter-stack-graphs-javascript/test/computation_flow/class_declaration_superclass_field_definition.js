class Foo {
    bar;
    baz = 1;
    static quux;
    static doo = 1;
}

class Garply extends Foo { }

let obj = new Garply();

obj.bar;
//  ^ defined: 2

obj.baz;
//  ^ defined: 3

obj.quux;
//  ^ defined: 4

obj.doo;
//  ^ defined: 5