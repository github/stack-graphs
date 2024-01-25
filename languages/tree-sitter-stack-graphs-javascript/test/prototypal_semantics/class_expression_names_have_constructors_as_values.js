(class Foo {
    constructor(o) {
        return o;
    }

    bar() {
        let obj = {
            x: 1
        };

        Foo(obj).x;
        //       ^ defined: 8
    }
});