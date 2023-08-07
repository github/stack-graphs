let Foo = class {
    bar;
    baz = 1;
    static quux;
    static doo = 1;
};

let obj = new Foo();

obj.bar;
//  ^ defined: 2

obj.baz;
//  ^ defined: 3

obj.quux;
//  ^ defined: 4

obj.doo;
//  ^ defined: 5