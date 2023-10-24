let Foo = class {
    bar() {

    }
};

let Baz = class extends Foo { };

let obj = new Baz();
obj.bar;
//  ^ defined: 2