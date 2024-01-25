// We can't simply say that Foo is defined on line 5 because the _name_ isn't,
// but we can verify that the constructor is in fact wired up as a function
// value to the name like so:

let Foo = class {
    constructor(o) {
        return o;
    }
};

let obj = {
    x: 1
};

Foo(obj).x;
//       ^ defined: 12