let Foo = class {
    bar = 1;
    baz() {

    }
};

Foo.bar;
//  ^ defined:
// bar should not be visible here

Foo.baz;
//  ^ defined:
// baz should not be visible here